# Epic 4 — Connection Crate

**Objective:** Implement the connection loop — the single `go!` coroutine running an epoll loop that manages TCP I/O, receives commands from application coroutines, and dispatches responses. This is the most may-specific part of the codebase.

**Dependencies:** Epic 0 (scaffolding) + Epic 1 (base) + Epic 2 (codec) + Epic 3 (protocol)

**Source docs:** `docs/04-system-design.md`, `docs/06-connection-layer-design.md`, `docs/02-may_postgres_comparison.md`

## Crate Overview

```mermaid
graph TB
    subgraph "connection crate — may + epoll + TCP"
        subgraph "Connection Loop (go! coroutine)"
            Loop[epoll connection_loop]
            ReadBuf[read_buf: BytesMut]
            WriteBuf[write_buf: BytesMut]
            RespBuf[response_queue: VecDeque]
            Codec[RESP Reader]
        end
        
        Input[mpsc Queue<Request>] --> InputProc[process queued requests]
        InputProc --> WriteBuf
        WriteBuf --> FlushWrite[flush to socket]
        FlushWrite --> ReadFromSocket[read from socket]
        ReadFromSocket --> ReadBuf
        ReadBuf --> Decode[decode via Codec]
        Decode --> RespBuf
        RespBuf --> Dispatch[dispatch via spsc]
        Dispatch -. sends to.> Apps[Application Coroutines]
        
        Loop -. runs.-> InputProc
        Loop -. runs.-> FlushWrite
        Loop -. runs.-> ReadFromSocket
        Loop -. runs.-> Decode
        Loop -. runs.-> Dispatch
    end
    
    subgraph "External deps"
        Bytes[bytes]
        Log[log]
        May[may — go!, WaitIo, Queue, spsync]
        Socket2[socket2 — TCP config]
    end
    
    Bytes -. used by.-> Loop
    Log -. used by.-> Loop
    May -. used by.-> Loop
    May -. used by.-> Input
    May -. used by.-> Dispatch
    Socket2 -. used by.-> Connect[TcpConnector]
```

## Connection Loop Algorithm

```mermaid
flowchart TD
    Start([Start Connection Loop]) --> Init[init read_buf, write_buf, resp_queue]
    Init --> Loop[main epoll loop]
    
    Loop --> HasRequests{any pending<br/>requests?}
    HasRequests -->|yes| ProcessReqs[pop Request<br/>add to resp_queue<br/>append to write_buf]
    HasRequests -->|no| CheckWrite{write_buf<br/>has data?}
    
    ProcessReqs --> FlushWrite[flush write_buf<br/>to socket nonblock]
    FlushWrite --> CheckWrite
    
    CheckWrite -->|has data| TryWrite{write complete?}
    CheckWrite -->|empty| WaitEpoll[epoll_wait]
    
    TryWrite -->|yes| ReadFromSocket[read from socket<br/>into read_buf nonblock]
    TryWrite -->|no (WouldBlock)| WaitEpoll
    
    ReadFromSocket --> HasData{data available?}
    HasData -->|yes| DecodeMessages[decode buffered<br/>data into responses]
    HasData -->|no| WaitEpoll
    
    DecodeMessages --> DispatchResp[dispatch to<br/>resp_queue VecDeque]
    DispatchResp --> WaitEpoll
    
    WaitEpoll -->|READABLE| ReadFromSocket
    WaitEpoll -->|WRITABLE| HasRequests
    WaitEpoll -->|BOTH| HasRequests
    WaitEpoll -->|CLOSED| CloseConn[close connection]
    
    CloseConn --> End([End Loop])
```

## Implementation Order

```mermaid
flowchart LR
    S0[Epic Overview] --> S1[Story 4.1<br/>TcpConnector<br/>TCP connect]
    S1 --> S2[Story 4.2<br/>Connection struct<br/>request queue]
    S2 --> S3[Story 4.3<br/>epoll loop body<br/>I/O + dispatch]
    S3 --> S4[Story 4.4<br/>Integration<br/>end-to-end test]
    S4 --> PASS[All tests pass<br/>cargo test -p connection]
```
