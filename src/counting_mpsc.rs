use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};


pub struct Sender<T> {
    mpsc_sender : mpsc::Sender<T>,
    count_ref   : Arc<AtomicUsize>,
}

pub struct Receiver<T> {
    mpsc_reciever : mpsc::Receiver<T>,
    count_ref     : Arc<AtomicUsize>,
}

pub struct ReceiverCount {
    count_ref : Arc<AtomicUsize>,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let count_ref = Arc::new(AtomicUsize::new(0));
    let count_ref_clone = count_ref.clone(); 
    let (mpsc_sender, mpsc_receiver) = mpsc::channel();

    (
        Sender {
            mpsc_sender : mpsc_sender,
            count_ref : count_ref,
        },
        Receiver {
            mpsc_reciever : mpsc_receiver,
            count_ref : count_ref_clone,
        }
    )
}

impl <T> Sender<T> {
    pub fn send(&mut self, item : T) -> Result<(), mpsc::SendError<T>> { 
        match self.mpsc_sender.send(item) {
            Ok(x) => {
                self.count_ref.fetch_add(1, Ordering::SeqCst);
                return Ok(x);
            }
            x => {
                return x;
            }
        }
    }

    pub fn get_count(&self) -> usize {
        return self.count_ref.load(Ordering::SeqCst);
    }
}

impl <T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            mpsc_sender : self.mpsc_sender.clone(),
            count_ref   : self.count_ref.clone(),
        }
    }
}

impl <T> Receiver<T> {
    pub fn recv(&mut self) -> Result<T, mpsc::RecvError> {
        match self.mpsc_reciever.recv() {
            Ok(x) => {
                self.count_ref.fetch_sub(1, Ordering::SeqCst);
                return Ok(x);
            }
            x => {
                return x;
            }
        }
    }

    pub fn try_recv(&mut self) -> Result<T, mpsc::TryRecvError> {
        match self.mpsc_reciever.try_recv() {
            Ok(x) => {
                self.count_ref.fetch_sub(1, Ordering::SeqCst);
                return Ok(x);
            }
            x => {
                return x;
            }
        }
    }

    pub fn get_count(&self) -> usize {
        return self.count_ref.load(Ordering::SeqCst);
    }

    pub fn clone_count(&self) -> ReceiverCount {
        ReceiverCount { count_ref : self.count_ref.clone() }
    }
}

impl ReceiverCount {
    pub fn get_count(&self) -> usize {
        return self.count_ref.load(Ordering::SeqCst);
    }
}

