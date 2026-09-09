use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct RefreshWorker {
    commands: Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

enum Command {
    Wake,
    Stop,
}

impl RefreshWorker {
    pub fn start(
        mut interval: impl FnMut() -> Duration + Send + 'static,
        mut tick: impl FnMut() + Send + 'static,
    ) -> Self {
        let (commands, receiver) = mpsc::channel();
        let thread = thread::spawn(move || loop {
            match receiver.recv_timeout(interval()) {
                Ok(Command::Wake) | Err(RecvTimeoutError::Timeout) => tick(),
                Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            }
        });
        Self {
            commands,
            thread: Some(thread),
        }
    }

    pub fn wake(&self) {
        let _ = self.commands.send(Command::Wake);
    }
}

impl Drop for RefreshWorker {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            if let Err(error) = thread.join() {
                eprintln!("NeoHtop refresh worker failed: {error:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_runs_and_drop_joins_without_waiting_for_next_interval() {
        let (send, receive) = mpsc::channel();
        let mut first = true;
        let worker = RefreshWorker::start(
            move || {
                let duration = if first {
                    Duration::ZERO
                } else {
                    Duration::from_secs(30)
                };
                first = false;
                duration
            },
            move || send.send(()).unwrap(),
        );
        receive.recv_timeout(Duration::from_secs(5)).unwrap();
        let before = std::time::Instant::now();
        drop(worker);
        assert!(before.elapsed() < Duration::from_secs(5));
        assert_eq!(
            receive.recv_timeout(Duration::from_secs(5)),
            Err(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn wake_interrupts_the_current_interval() {
        let (send, receive) = mpsc::channel();
        let worker =
            RefreshWorker::start(|| Duration::from_secs(30), move || send.send(()).unwrap());

        worker.wake();

        receive.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}
