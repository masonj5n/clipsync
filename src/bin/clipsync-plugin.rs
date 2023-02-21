use anyhow::anyhow;
use clipsync_rpc::clipsync_client::ClipsyncClient;
use clipsync_rpc::{Yank, YankUpdateReq};
use log::{error, info, warn};
use neovim_lib::{Handler, Neovim, NeovimApi, RequestHandler, Session, Value};
use tokio::sync::mpsc;
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

#[derive(Debug)]
enum Action {
    Echo(String),
}

struct EventHandler {
    action_sender: mpsc::Sender<Action>,
    cs_client: Option<(ClipsyncClient<Channel>, String)>,
    t_handle: tokio::runtime::Handle,
}

impl EventHandler {
    async fn new(sender: mpsc::Sender<Action>) -> EventHandler {
        EventHandler {
            action_sender: sender,
            cs_client: None,
            t_handle: tokio::runtime::Handle::current(),
        }
    }
}

impl Handler for EventHandler {
    fn handle_notify(&mut self, _name: &str, _args: Vec<Value>) {}
}

impl RequestHandler for EventHandler {
    fn handle_request(&mut self, event: &str, values: Vec<Value>) -> Result<Value, Value> {
        match Messages::try_from((event, values)) {
            Ok(Messages::Yank(v)) => {
                info!("executing yank message: {v}");
                if let Some((client, _)) = &mut self.cs_client {
                    self.t_handle
                        .block_on(client.yank_update(YankUpdateReq {
                            yank: Some(Yank { contents: v }),
                        }))
                        .unwrap();
                    Ok(Value::String("Ok".into()))
                } else {
                    Ok(Value::String("Yank error".into()))
                }
            }
            Ok(Messages::Unknown(event, value)) => {
                warn!("received unknown event type: {event} with value: {value}");
                Ok(Value::String("received unknown event type".into()))
            }
            Ok(Messages::Connect { address }) => {
                info!("executing connect message: {}", address.uri().to_string());
                match self
                    .t_handle
                    .block_on(ClipsyncClient::connect(*address.clone()))
                {
                    Ok(client) => {
                        self.cs_client = Some((client, address.uri().to_string()));
                        Ok(Value::String("Ok".into()))
                    }
                    Err(e) => {
                        error!("failed to connect to client: {e}");
                        self.action_sender
                            .blocking_send(Action::Echo(format!(
                                "failed to connect to client: {e}"
                            )))
                            .unwrap();
                        Ok(Value::String("Error connecting".into()))
                    }
                }
            }
            Ok(Messages::Disconnect) => {
                info!("executing disconnect message");
                self.cs_client = None;
                Ok(Value::String("Ok".into()))
            }
            Ok(Messages::Status) => {
                info!("executing status message");
                match &self.cs_client {
                    Some((_, c)) => Ok(Value::String(format!("connected to {}", c).into())),
                    None => Ok(Value::String("disconnected".into())),
                }
            }
            Err(e) => {
                error!("couldn't convert message into Message: {e}");
                Ok(Value::String(
                    "couldn't convert message into known message type".into(),
                ))
            }
        }
    }
}

#[tokio::main]
async fn main() {
    simple_logging::log_to_file("/tmp/clipsync_log.txt", log::LevelFilter::Info).unwrap();

    let (sender, mut receiver) = mpsc::channel(10);
    let event_handler = EventHandler::new(sender).await;

    let mut session = Session::new_parent().unwrap();
    session.start_event_loop_handler(event_handler);

    let mut nvim = Neovim::new(session);

    while let Some(action) = receiver.recv().await {
        match action {
            Action::Echo(m) => nvim.command(&format!("echo \"{}\"", m)).unwrap(),
        };
    }
}

impl TryFrom<(&str, Vec<Value>)> for Messages {
    type Error = anyhow::Error;

    fn try_from(event_and_values: (&str, Vec<Value>)) -> Result<Self, Self::Error> {
        match (event_and_values.0.as_ref(), event_and_values.1) {
            ("yank", v) => {
                info!("received yank message");
                if let Value::String(contents) = v.first().unwrap() {
                    Ok(Messages::Yank(contents.to_string()))
                } else {
                    Err(anyhow!("Failed to parse yanked string"))
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
                    Err(anyhow!("failed to parse connect string into address"))
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
}
