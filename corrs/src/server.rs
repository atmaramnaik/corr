use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{mpsc};
use crate::hub::{Hub, User};
use warp::Filter;
use warp::ws::WebSocket;
use futures::{FutureExt, StreamExt};
use corr_lib::core::proto::{Output};
use corr_lib::core::runtime::Client;
use crate::Config;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
pub struct Server{
    port: u16,
    hub:Arc<Hub>
}
impl Server {
    pub fn new(port: u16) -> Self {
        Server {
            port,
            hub:Arc::new(Hub::new())
        }
    }
    pub async fn run(&self,config:Config) {
        let hub = self.hub.clone();

        let runner = warp::path("api")
            .and(warp::ws())
            .and(warp::any().map(move || hub.clone()))
            .map(
                move |ws: warp::ws::Ws,
                      hub: Arc<Hub>| {
                        ws
                        .on_upgrade(move |web_socket| async move {
                            tokio::spawn(Self::user_connected(hub,web_socket));
                        })
                },
            );

        let web = warp::path::end()
            .and(warp::fs::dir(config.wroot.clone()));
        let web_runner = warp::path("runner")
            .and(warp::fs::dir(config.wroot.clone()));
        let site=runner.or(web).or(web_runner);

        let shutdown = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install CTRL+C signal handler");
        };
        let (_, serving) =
            warp::serve(site).bind_with_graceful_shutdown(([0, 0, 0, 0], self.port), shutdown);


        tokio::select! {
            _ = serving => {}
        }
    }
    async fn user_connected(
        hub: Arc<Hub>,
        web_socket: WebSocket,
    ) {
        let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

        eprintln!("User Connected: {}", my_id);

        // Split the socket into a sender and receive of messages.
        let (user_ws_tx, user_ws_rx) = web_socket.split();
        let (tx, rx) = mpsc::channel(100);
        let rx = tokio_stream::wrappers::ReceiverStream::new(rx);
        tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
            if let Err(e) = result {
                eprintln!("websocket send error: {}", e);
            }
        }));
        let user = User::new(tx,user_ws_rx);
        user.send(Output::new_connected(format!("You are now connected to corrs server!!"))).await;
        hub.start(user).await
    }
}