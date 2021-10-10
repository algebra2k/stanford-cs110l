use crossbeam_channel;
use std::sync::{Arc, Mutex};
use std::{thread, time};

fn parallel_map_channel<T, U, F>(input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + Copy + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    for _ in 0..input_vec.len() {
        output_vec.push(Default::default());
    }

    let (sender, rerceiver): (
        crossbeam_channel::Sender<(usize, T)>,
        crossbeam_channel::Receiver<(usize, T)>,
    ) = crossbeam_channel::unbounded();

    let (output_sender, output_receiver): (
        crossbeam_channel::Sender<(usize, U)>,
        crossbeam_channel::Receiver<(usize, U)>,
    ) = crossbeam_channel::unbounded();

    let mut handles = Vec::new();
    for _ in 0..num_threads {
        let output_sender = output_sender.clone();
        let receiver = rerceiver.clone();
        let handle = thread::spawn(move || {
            while let Ok((i, v)) = receiver.recv() {
                output_sender
                    .send((i, f(v)))
                    .expect("Tried writing to channel, but there are no receivers!");
            }
        });
        handles.push(handle);
    }

    for (i, v) in input_vec.iter().enumerate() {
        sender
            .send((i, *v))
            .expect("Tried writing to channel, but there are no receivers!");
    }
    drop(sender);

    for handle in handles {
        handle.join().unwrap();
    }
    drop(output_sender);

    while let Ok((i, v)) = output_receiver.recv() {
        output_vec[i] = v;
    }
    output_vec
}

fn parallel_map<T, U, F>(input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + Copy + 'static,
    U: Send + std::fmt::Debug + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    for _ in 0..input_vec.len() {
        output_vec.push(Default::default());
    }
    let shared_output_vec: Arc<Mutex<Vec<U>>> = Arc::new(Mutex::new(output_vec));

    let (sender, rerceiver): (
        crossbeam_channel::Sender<(usize, T)>,
        crossbeam_channel::Receiver<(usize, T)>,
    ) = crossbeam_channel::unbounded();

    let mut handles = Vec::new();
    for _ in 0..num_threads {
        // let sender = sender2.clone();
        let receiver = rerceiver.clone();
        let shared_output_vec = shared_output_vec.clone();
        let handle = thread::spawn(move || {
            while let Ok((i, v)) = receiver.recv() {
                let mut lock_vec;
                {
                    lock_vec = shared_output_vec.lock().unwrap();
                }
                // here, not need lock.
                lock_vec[i] = f(v);
            }
        });
        handles.push(handle);
    }

    for (i, v) in input_vec.iter().enumerate() {
        sender
            .send((i, *v))
            .expect("Tried writing to channel, but there are no receivers!");
    }
    drop(sender);

    for handle in handles {
        handle.join().unwrap();
    }
    let lock = Arc::try_unwrap(shared_output_vec).expect("Lock still has multiple owners");
    let output_vec = lock.into_inner().expect("Mutex cannot be locked");
    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v.clone(), 10, |num| {
        println!("{} squared is {} (shared by state)", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);

    let squares = parallel_map_channel(v.clone(), 10, |num| {
        println!("{} squared is {} (shared by channel)", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
