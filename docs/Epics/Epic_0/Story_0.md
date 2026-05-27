# Epic 0 — Scaffolding

**Objective:** Set up the workspace structure, Cargo.toml configuration, lint tooling, and documentation layout. This is the foundation that every subsequent epic depends on.

**Dependencies:** None (first epic)

**Source docs:** `docs/08-module-structure.md`, `docs/11-dependencies.md`, `docs/09-migration-guide.md`

## Workspace Goal

```mermaid
graph TB
    subgraph "Workspace: may_redis"
        WRoot[Workspace Cargo.toml<br/>6 members]
        
        subgraph "crates/"
            Base[base<br/>~150 LOC<br/>deps: bytes]
            Codec[codec<br/>~300 LOC<br/>deps: bytes, base]
            Proto[protocol<br/>~400 LOC<br/>deps: bytes, log, may, base, codec]
            Conn[connection<br/>~400 LOC<br/>deps: bytes, log, may, socket2, base, codec]
            Client[client<br/>~300 LOC<br/>deps: base, codec, protocol, connection]
            Umb[may-redis<br/>~50 LOC<br/>re-exports all]
        end
        
        WRoot --> Base
        WRoot --> Codec
        WRoot --> Proto
        WRoot --> Conn
        WRoot --> Client
        WRoot --> Umb
    end
```

## Dependency Graph

```mermaid
graph LR
    subgraph "External Dependencies"
        Bytes[bytes 1.7]
        Log[log 0.4]
        May[may 0.3]
        Socket2[socket2 0.5]
    end
    
    Base[base] --> Bytes
    Codec[codec] --> Bytes
    Codec --> Base
    Proto[protocol] --> Bytes
    Proto --> Log
    Proto --> May
    Proto --> Base
    Proto --> Codec
    Conn[connection] --> Bytes
    Conn --> Log
    Conn --> May
    Conn --> Socket2
    Conn --> Base
    Conn --> Codec
    Client[client] --> Base
    Client --> Codec
    Client --> Proto
    Client --> Conn
    Umb[may-redis] --> Base
    Umb --> Codec
    Umb --> Proto
    Umb --> Conn
    Umb --> Client
```

## Feature Flag Matrix

```mermaid
graph TD
    subgraph "Feature Flags"
        F_Base[base — always on]
        F_Codec[codec — always on]
        F_Protocol[protocol — always on]
        F_Connection[connection — default]
        F_Client[client — default]
        F_Test[test — off by default]
        F_Pool[pool — off by default]
    end
    
    subgraph "Build Configurations"
        Full[Full build<br/>base + codec + protocol + connection + client]
        BaseOnly[Minimal build<br/>base + codec only]
        TestOnly[Test build<br/>+ test helpers]
        Prod[Production<br/>same as full]
    end
    
    F_Base --> Full
    F_Codec --> Full
    F_Protocol --> Full
    F_Connection --> Full
    F_Client --> Full
    F_Base --> BaseOnly
    F_Codec --> BaseOnly
    F_Base --> TestOnly
    F_Codec --> TestOnly
    F_Protocol --> TestOnly
    F_Test --> TestOnly
    F_Base --> Prod
    F_Codec --> Prod
    F_Protocol --> Prod
    F_Connection --> Prod
    F_Client --> Prod
```
