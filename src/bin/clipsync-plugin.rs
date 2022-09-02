use clipsync_rpc::clipsync_client::ClipsyncClient;
use clipsync_rpc::{Yank, YankUpdateReq};
use log::{error, info, warn};
use neovim_lib::{Neovim, NeovimApi, Session, Value};
use tonic::transport::Channel;

pub mod clipsync_rpc {
    tonic::include_proto!("clipsync_rpc");
}

enum Messages {
    Connect {
        address: Box<tonic::transport::Endpoint>,
    },
    Disconnect,
    Yank(String),
    Status,
    Unknown(String, String),
}

impl TryFrom<(String, Vec<Value>)> for Messages {
    fn try_from(event_and_values: (String, Vec<Value>)) -> Result<Self, Self::Error> {
        match (event_and_values.0.as_ref(), event_and_values.1) {
            ("yank", v) => {
                info!("received yank message");
                if let Value::String(contents) = v.first().unwrap() {
                    Ok(Messages::Yank(contents.to_string()))
                } else {
                    Err(anyhow::anyhow!("Failed to parse yanked string"))
                }
            }

            ("connect", v) => {
                info!("Received connect message");
                let address = v
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<String>()
                    .replace('"', "");
                info!("received connection string: {address}");
                if let Ok(a) = tonic::transport::Endpoint::from_shared(address) {
                    Ok(Messages::Connect {
                        address: Box::new(a),
                    })
                } else {
                    Err(anyhow::anyhow!(
                        "failed to parse connect string into address"
                    ))
                }
            }

            ("disconnect", _) => {
                info!("received disconnect message");
                Ok(Messages::Disconnect)
            }

            ("status", _) => {
                info!("received status message");
                Ok(Messages::Status)
            }
            (e, v) => Ok(Messages::Unknown(
                e.to_string(),
                v.iter().map(|v| v.to_string()).collect::<String>(),
            )),
        }
    }

    type Error = anyhow::Error;
}

struct EventHandler {
    nvim: Neovim,
    cs_client: Option<(ClipsyncClient<Channel>, String)>,
}

impl EventHandler {
    fn new() -> EventHandler {
        let session = Session::new_parent().unwrap();
        let nvim = Neovim::new(session);

        EventHandler {
            nvim,
            cs_client: None,
        }
    }

    async fn recv(&mut self) {
        let receiver = self.nvim.session.start_event_loop_channel();

        for (event, values) in receiver {
            match Messages::try_from((event, values)) {
                Ok(Messages::Yank(v)) => {
                    info!("executing yank message: {v}");
                    if let Some((client, _)) = &mut self.cs_client {
                        client
                            .yank_update(YankUpdateReq {
                                yank: Some(Yank { contents: v }),
                            })
                            .await
                            .unwrap();
                    }
                }
                Ok(Messages::Unknown(event, value)) => {
                    warn!("received unknown event type: {event} with value: {value}");
                    continue;
                }
                Ok(Messages::Connect { address }) => {
                    info!("executing connect message: {}", address.uri().to_string());
                    match ClipsyncClient::connect(*address.clone()).await {
                        Ok(client) => self.cs_client = Some((client, address.uri().to_string())),
                        Err(e) => {
                            error!("failed to connect to client: {e}");
                            self.nvim
                                .command(&format!("echo \"failed to connect to client: {e}\""))
                                .unwrap();
                        }
                    };
                }
                Ok(Messages::Disconnect) => {
                    info!("executing disconnect message");
                    self.cs_client = None;
                }
                Ok(Messages::Status) => {
                    info!("executing status message");
                    match &self.cs_client {
                        Some(c) => self
                            .nvim
                            .command(&format!("echo \"connected to {}\"", c.1))
                            .unwrap(),
                        None => self.nvim.command("echo \"disconnected\"").unwrap(),
                    };
                }
                Err(e) => error!("couldn't convert message into Message: {e}"),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    simple_logging::log_to_file("/tmp/clipsync_log.txt", log::LevelFilter::Info).unwrap();
    let handle = tokio::task::spawn_blocking(move || async {
        let mut event_handler = EventHandler::new();
        event_handler.recv().await
    });
    handle.await.unwrap().await;
}
