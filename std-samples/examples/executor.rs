use std::future::Future;
use std::pin::Pin;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

struct Task {
    fut: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
    tx: mpsc::Sender<Arc<Task>>,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        let again = self.clone();
        self.tx.send(again).unwrap(); // re-queue ourselves
    }
}

pub struct Executor { rx: mpsc::Receiver<Arc<Task>> }

#[derive(Clone)]
pub struct Spawner { tx: mpsc::Sender<Arc<Task>> }

pub fn new_executor() -> (Executor, Spawner) {
    let (tx, rx) = mpsc::channel();
    (Executor { rx }, Spawner { tx })
}

impl Spawner {
    pub fn spawn(&self, f: impl Future<Output = ()> + Send + 'static) {
        let task = Arc::new(Task { fut: Mutex::new(Some(Box::pin(f))), tx: self.tx.clone() });
        self.tx.send(task).unwrap();
    }
}

impl Executor {
    /// Runs until every task has finished and every Spawner is dropped.
    pub fn run(self) {
        while let Ok(task) = self.rx.recv() {
            let mut slot = task.fut.lock().unwrap();
            if let Some(mut fut) = slot.take() {
                let waker = Waker::from(task.clone());
                let mut cx = Context::from_waker(&waker);
                if fut.as_mut().poll(&mut cx).is_pending() {
                    *slot = Some(fut); // not done: put it back until woken
                }
            }
        }
    }
}

struct TimerState { done: bool, waker: Option<Waker> }

pub struct Timer { state: Arc<Mutex<TimerState>> }

impl Timer {
    pub fn new(d: Duration) -> Self {
        let state = Arc::new(Mutex::new(TimerState { done: false, waker: None }));
        let s = state.clone();
        thread::spawn(move || {
            thread::sleep(d);
            let mut g = s.lock().unwrap();
            g.done = true;
            if let Some(w) = g.waker.take() { w.wake(); }
        });
        Timer { state }
    }
}

impl Future for Timer {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut g = self.state.lock().unwrap();
        if g.done {
            Poll::Ready(())
        } else {
            g.waker = Some(cx.waker().clone()); // store the latest waker on every poll
            Poll::Pending
        }
    }
}

fn main() {
    let (exec, spawner) = new_executor();
    let start = Instant::now();
    for id in 0..3u64 {
        spawner.spawn(async move {
            Timer::new(Duration::from_millis(100 * (id + 1))).await;
            println!("task {id} done at {:?}", start.elapsed());
        });
    }
    drop(spawner); // otherwise run() would wait for more work forever
    exec.run();
}
