# Connection Layer Design — The Connection Loop

## Overview

The connection layer is the heart of may_redis — it runs a single `go!` coroutine that:
1. Receives commands from application coroutines via an mpsc request queue
2. Reads/writes the TCP socket using epoll
3. Dispatches responses back to the correct application coroutine via spsc channels

```mermaid
graph TB
    subgraph "Application Coroutines"
        App1[App Coroutine 1\n is_seen(jti-abc)]
        App2[App Coroutine 2\n record(jti-def)]
        App3[App Coroutine 3\n get(key)]
    end
    
    subgraph "Request Queue"
        Queue[Arc~Queue~Request>]\n mpsc Queue]
        Waker[WaitIoWaker]
    end
    
    subgraph "Connection Coroutine"
        Loop[epoll connection_loop]
        ReadBuf[read_buf: BytesMut]
        WriteBuf[write_buf: BytesMut]
        RespBuf[response_queue: VecDeque~Response~]
        Codec[RESP Codec]
    end
    
    subgraph "Transport"
        TCP[TcpStream\n may::net]
    end
    
    App1 -->|request.push()| Queue
    App2 -->|request.push()| Queue
    App3 -->|request.push()| Queue
    
    Queue -->|pop| Loop
    Loop -->|signal| Waker
    
    Loop -->|read| TCP
    TCP -->|data| Loop
    Loop -->|write| TCP
    
    Loop -->|decode| Codec
    Codec -->|dispatch| RespBuf
    RespBuf -->|tx.send()| App1
    RespBuf -->|tx.send()| App2
    RespBuf -->|tx.send()| App3
```

## Core Types

### Request

```
struct Request {
    tag: usize,                    // Sequence number for matching response
    command: BytesMut,             // Encoded RESP command
    sender: spsc::Sender<RedisResponse>,  // Response channel for this request
}
```

The `tag` is a monotonically increasing counter from a shared cell. Each request
gets a unique tag. The connection loop tags each response with the same tag,
so responses can be dispatched to the correct channel even when pipelined.

### Response

```
struct Response {
    tag: usize,
    tx: spsc::Sender<RedisResponse>,
}
```

### Connection

```
struct Connection {
    io_handle: JoinHandle<()>,    // The go! coroutine
    req_queue: Arc<Queue<Request>>, // Shared request queue
    waker: WaitIoWaker,           // Epoll signal to wake connection loop
    id: usize,                    // For debugging/logging
}
```

## Connection Loop Algorithm

```mermaid
flowchart TD
    Start([Start Connection Loop]) --> Init[init read_buf, write_buf, resp_queue]
    Init --> Loop[main epoll loop]
    
    Loop --> ProcessReq{process queued requests?}
    ProcessReq -->|yes| WriteCmds[write commands to write_buf]
    ProcessReq -->|no| CheckWrite{write_buf empty?}
    
    WriteCmds --> FlushWrite[flush write_buf to socket]
    FlushWrite --> CheckWrite
    
    CheckWrite -->|has data| ReadFromSocket[read from socket into read_buf]
    CheckWrite -->|empty| WaitEpoll
    
    ReadFromSocket --> DecodeMessages{decode messages?}
    DecodeMessages -->|has messages| DispatchResp[dispatch to response_queue]
    DecodeMessages -->|none| WaitEpoll
    
    DispatchResp --> WaitEpoll
    
    WaitEpoll[epoll_wait] -->|READABLE| ReadFromSocket
    WaitEpoll -->|WRITABLE| ProcessReq
    WaitEpoll -->|TIMEOUT| ProcessReq
    WaitEpoll -->|CLOSED| CloseConn[close connection]
    
    CloseConn --> End([End Loop])
    
    ProcessReq -->|no requests| WaitEpoll
    FlushWrite -->|partial write| CheckWrite
```

### Detailed Loop Body

```
fn connection_loop(stream: TcpStream, req_queue: Arc<Queue<Request>>) {
    let mut read_buf = BytesMut::with_capacity(4096 * 2);
    let mut write_buf = BytesMut::with_capacity(4096 * 2);
    let mut resp_queue = VecDeque::with_capacity(512);
    
    loop {
        // 1. Process incoming requests from the queue
        while let Some(req) = req_queue.pop() {
            let response = Response { tag: req.tag, tx: req.tx };
            resp_queue.push_back(response);
            write_buf.extend_from_slice(&req.command);
        }
        
        // 2. Non-blocking write
        nonblock_write(&mut stream, &mut write_buf);
        
        // 3. Non-blocking read
        let read_blocked = nonblock_read(&mut stream, &mut read_buf);
        
        // 4. Decode buffered data into responses
        decode_messages(&mut read_buf, &mut resp_queue);
        
        // 5. epoll_wait for next event
        //    - If read not blocked: poll with READABLE priority
        //    - If write has data: poll with WRITABLE priority
        //    - If no pending data: poll with READABLE (expecting new requests)
    }
}
```

### Non-Blocking I/O

The same non-blocking read/write pattern as may_postgres:

```
fn nonblock_read(stream: &mut TcpStream, read_buf: &mut BytesMut) -> bool {
    let remaining = read_buf.capacity() - read_buf.len();
    let buf: &mut [u8] = unsafe { transmute(read_buf.chunk_mut()) };
    let mut read_cnt = 0;
    
    while read_cnt < remaining {
        match stream.read(unsafe { buf.get_unchecked_mut(read_cnt..) }) {
            Ok(0) => return false,  // Connection closed
            Ok(n) => read_cnt += n,
            Err(e) if e.kind() == WouldBlock => break,  // No more data
            Err(e) => return Err(e),
        }
    }
    
    unsafe { read_buf.advance_mut(read_cnt) };
    read_cnt < remaining  // true = still more data available
}
```

## Epoll Event Handling

```mermaid
stateDiagram-v2
    [*] --> ExpectingRead
    ExpectingRead --> ProcessingRequests: epoll(WRITABLE)
    ProcessingRequests --> WriteToSocket
    WriteToSocket --> ReadFromSocket: write nonblock OK
    ReadFromSocket --> ExpectingRead: read_blocked=true
    ReadFromSocket --> ProcessingRequests: data available
    
    ExpectingRead --> ReadFromSocket: epoll(READABLE)
    ReadFromSocket --> DecodeAndDispatch
    DecodeAndDispatch --> ExpectingRead
```

The epoll events drive the state machine:
- **READABLE** → try to read from socket, decode buffer, dispatch responses
- **WRITABLE** → pop more requests from queue, write to socket
- **Both** → process writes first (flush pending data), then read

## Request-Response Matching

```mermaid
sequenceDiagram
    participant A1 as App Coroutine 1
    participant Q as req_queue
    participant C as Connection Loop
    participant A2 as App Coroutine 2
    participant R as resp_queue
    
    A1->>Q: push Request(tag=1, cmd="GET foo", tx=tx1)
    A2->>Q: push Request(tag=2, cmd="SET bar 123", tx=tx2)
    
    Note over C: epoll(WRITABLE)
    C->>Q: pop Request(tag=1)
    C->>R: push Response(tag=1, tx=tx1)
    C->>C: write_buf += "GET foo"
    
    C->>Q: pop Request(tag=2)
    C->>R: push Response(tag=2, tx=tx2)
    C->>C: write_buf += "SET bar 123"
    
    C->>C: flush write_buf to socket
    
    Note over C: epoll(READABLE)
    C->>C: read "*1\r\n:1\r\n" from socket
    
    C->>C: decode -> integer 1
    C->>R: find Response(tag=2)
    C->>tx2: send Ok(1)
    
    C->>C: read "*1\r\n$3\r\nbaz\r\n" from socket
    
    C->>C: decode -> bulk "baz"
    C->>R: find Response(tag=1)
    C->>tx1: send Ok("baz")
    
    tx1-->>A1: Ok("baz")
    tx2-->>A2: Ok(1)
```

The tag ensures correct response ordering regardless of which coroutine sends first.

