use std::future::Future;

use glib::MainContext;

pub fn async_loop_do_mainthread<R: 'static, F: Future<Output = R> + 'static + std::marker::Copy>(f: F) {
    MainContext::ref_thread_default().spawn_local(async move {
        loop {
            f.await;
        }
    });
}