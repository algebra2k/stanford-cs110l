mod rate_limiting;
mod request;
mod response;

use clap::Clap;
use rand::{Rng, SeedableRng};
// use std::net::{TcpListener, TcpStream};
// use std::sync::{Arc, Mutex};
// use threadpool::ThreadPool;
// use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::{task, time};

use crate::rate_limiting::FixWindowRateLimit;

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Clap, Debug)]
#[clap(about = "Fun with load balancing")]
struct CmdOptions {
    #[clap(
        short,
        long,
        about = "IP/port to bind to",
        default_value = "0.0.0.0:1100"
    )]
    bind: String,
    #[clap(short, long, about = "Upstream host to forward requests to")]
    upstream: Vec<String>,
    #[clap(
        long,
        about = "Perform active health checks on this interval (in seconds)",
        default_value = "10"
    )]
    active_health_check_interval: usize,
    #[clap(
        long,
        about = "Path to send request to for active health checks",
        default_value = "/"
    )]
    active_health_check_path: String,
    #[clap(
        long,
        about = "Maximum number of requests to accept per IP per minute (0 = unlimited)",
        default_value = "0"
    )]
    max_requests_per_minute: usize,
}

enum UpstreamState {
    Active,
    Dead,
}

struct Upstream {
    address: String,
    state: UpstreamState,
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    upstream_addresses: Arc<Mutex<Vec<Upstream>>>,
}

impl ProxyState {
    pub async fn select_upstream(&mut self) -> Option<TcpStream> {
        // check all upstream and change state one.
        let mut upstreams = self.upstream_addresses.lock().await;
        for i in 0..upstreams.len() {
            if matches!(upstreams[i].state, UpstreamState::Dead) {
                continue;
            }
            let address = &upstreams[i].address;
            match TcpStream::connect(address).await {
                Ok(_stream) => {
                    upstreams[i].state = UpstreamState::Active;
                    // return Some(stream);
                }
                Err(err) => {
                    log::error!("Failed to connect to upstream {}: {}", address, err);
                    upstreams[i].state = UpstreamState::Dead;
                    // continue;
                }
            }
        }

        // filter upstream with active upstream
        let active_upstreams = upstreams
            .iter()
            .filter(|upstream| matches!(upstream.state, UpstreamState::Active))
            .collect::<Vec<&Upstream>>();

        // random select active upstream
        let mut rng = rand::rngs::StdRng::from_entropy();
        let upstream_idx = rng.gen_range(0, active_upstreams.len());
        let upstream_ip = &active_upstreams[upstream_idx].address;
        match TcpStream::connect(upstream_ip).await {
            Ok(stream) => Some(stream),
            Err(_err) => None,
        }
    }
}

async fn run_health_check_interval(shared_state: Arc<Mutex<ProxyState>>) {
    tokio::spawn(async move {
        let mut interval;
        let active_health_check_path;
        {
            let state = shared_state.lock().await;
            interval = time::interval(Duration::from_secs(
                state.active_health_check_interval as u64,
            ));
            active_health_check_path = state.active_health_check_path.clone();
        }
        loop {
            interval.tick().await; // wait for 0s - zero wait here
            interval.tick().await; // wait for interval
            let len;
            {
                let state = shared_state.lock().await;
                len = state.upstream_addresses.lock().await.len();
            }
            let mut tasks = Vec::new();
            for i in 0..len {
                let shared_upstream_vec;
                {
                    let state = shared_state.lock().await;
                    shared_upstream_vec = state.upstream_addresses.clone()
                };
                let active_health_check_path = active_health_check_path.clone();
                tasks.push(task::spawn(async move {
                    let mut upstream = shared_upstream_vec.lock().await;
                    let request = http::Request::builder()
                        .method(http::Method::GET)
                        .uri(active_health_check_path)
                        .header("host", &upstream[i].address)
                        .body(Vec::new())
                        .unwrap();

                    let mut stream = match TcpStream::connect(&upstream[i].address).await {
                        Ok(stream) => stream,
                        Err(err) => {
                            log::error!(
                                "Failed to connect to upstream {}: {}",
                                upstream[i].address,
                                err
                            );
                            // upstream[i].state = UpstreamState::Dead;
                            return;
                        }
                    };

                    if let Err(_error) = request::write_to_stream(&request, &mut stream).await {
                        // upstream[i].state = UpstreamState::Dead;
                        return;
                    };
                    match response::read_from_stream(&mut stream, request.method()).await {
                        Ok(response) => {
                            if matches!(response.status(), http::StatusCode::OK) {
                                println!("receive response");
                                upstream[i].state = UpstreamState::Active;
                            } else {
                                upstream[i].state = UpstreamState::Dead;
                            }
                        }
                        Err(_) => upstream[i].state = UpstreamState::Dead,
                    }
                }));
            }
            for task in tasks {
                task.await.unwrap();
            }
        }
    });
}

#[tokio::main]
async fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let mut listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    // Handle incoming connections
    let mut upstream_state = Vec::new();
    for it in options.upstream.iter() {
        upstream_state.push(Upstream {
            address: it.clone(),
            state: UpstreamState::Active,
        });
    }

    let state = ProxyState {
        upstream_addresses: Arc::new(Mutex::new(upstream_state)),
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
    };

    let shared_rate_limit: Arc<Mutex<FixWindowRateLimit>> = Arc::new(Mutex::new(
        FixWindowRateLimit::new(state.max_requests_per_minute.clone()),
    ));
    let share_state: Arc<Mutex<ProxyState>> = Arc::new(Mutex::new(state));
    // let num_threads = num_cpus::get();
    // let thread_pool = ThreadPool::new(num_threads);

    // for stream in listener.incoming() {
    //     if let Ok(stream) = stream {
    //         // Handle the connection!
    //         // handle_connection(stream, &state);
    //         dispatch_connection_handle(&thread_pool, stream, share_state.clone());
    //     }
    // }
    run_health_check_interval(share_state.clone()).await;
    loop {
        match listener.accept().await {
            Ok((stream, _sock_addr)) => {
                // task::spawn(async );
                dispatch_connection_handle(stream, share_state.clone(), shared_rate_limit.clone())
                    .await;
            }
            Err(e) => {
                println!("couldn't get client: {:?}", e);
            }
        }
    }
}

// async fn connect_to_upstream(state: &ProxyState) -> Result<TcpStream, std::io::Error> {
//     let mut rng = rand::rngs::StdRng::from_entropy();
//     let upstream_idx = rng.gen_range(0, state.upstream_addresses.len());
//     let upstream_ip = &state.upstream_addresses[upstream_idx].address;
//     TcpStream::connect(upstream_ip).await.or_else(|err| {
//         log::error!("Failed to connect to upstream {}: {}", upstream_ip, err);
//         Err(err)
//     })
//     // TODO: implement failover (milestone 3)
// }

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!(
        "{} <- {}",
        client_ip,
        response::format_response_line(&response)
    );
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

// fn dispatch_connection_handle(
//     thread_pool: &ThreadPool,
//     client_conn: TcpStream,
//     share_state: Arc<Mutex<ProxyState>>,
// ) {
//     thread_pool.execute(move || {
//         let state;
//         {
//             state = share_state.lock().unwrap();
//         }
//         handle_connection(client_conn, &state);
//     })
// }

async fn dispatch_connection_handle(
    client_conn: TcpStream,
    share_state: Arc<Mutex<ProxyState>>,
    rate_limit: Arc<Mutex<FixWindowRateLimit>>,
) {
    tokio::spawn(async move { handle_connection(client_conn, share_state, rate_limit).await })
        .await
        .unwrap();
}
async fn handle_connection(
    mut client_conn: TcpStream,
    share_state: Arc<Mutex<ProxyState>>,
    share_rate_limit: Arc<Mutex<FixWindowRateLimit>>,
) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);

    // Open a connection to a random destination server
    // let mut upstream_conn = match connect_to_upstream(state).await {
    //     Ok(stream) => stream,
    //     Err(_error) => {
    //         let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
    //         send_response(&mut client_conn, &response).await;
    //         return;
    //     }
    // };
    let mut state = share_state.lock().await;
    let mut upstream_conn = match state.select_upstream().await {
        Some(stream) => stream,
        None => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            drop(state);
            return;
        }
    };
    drop(state);

    let upstream_ip = client_conn.peer_addr().unwrap().ip().to_string();
    let mut rate_limit;
    {
        rate_limit = share_rate_limit.lock().await;
    }
    if rate_limit.rate_limit(upstream_ip.as_str()).await {
        let response = response::make_http_error(http::StatusCode::TOO_MANY_REQUESTS);
        send_response(&mut client_conn, &response).await;
        return;
    }
    // The cliet may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn).await {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };
        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!(
                "Failed to send request to upstream {}: {}",
                upstream_ip,
                error
            );
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await
        {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}
