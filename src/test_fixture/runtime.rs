#![cfg(feature = "test")]
//! Run Docker/bollard work on a dedicated thread with its own Tokio runtime.

pub(super) fn run_on_docker_thread<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .name("may-redis-docker".into())
        .spawn(f)
        .expect("spawn docker fixture thread")
        .join()
        .expect("docker fixture thread panicked")
}

pub(super) fn block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for docker fixture");
    rt.block_on(future)
}
