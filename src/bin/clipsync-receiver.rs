use clipboard::{ClipboardContext, ClipboardProvider};
use clipsync_rpc::clipsync_server::{Clipsync, ClipsyncServer};
use clipsync_rpc::{YankUpdateReq, YankUpdateResp};
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};

pub mod clipsync_rpc {
    tonic::include_proto!("clipsync_rpc");
}

struct MyClipsync {
    clipboard: Mutex<ClipboardContext>,
}

#[tonic::async_trait]
impl Clipsync for MyClipsync {
    async fn yank_update(
        &self,
        request: Request<YankUpdateReq>,
    ) -> Result<Response<YankUpdateResp>, Status> {
        let yank = snailquote::unescape(&request.into_inner().yank.unwrap().contents).unwrap();
        self.clipboard.lock().await.set_contents(yank).unwrap();
        Ok(Response::new(YankUpdateResp {
            response: "success".to_string(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:8089".parse()?;
    let ctx = ClipboardProvider::new().unwrap();
    let greeter = MyClipsync {
        clipboard: Mutex::new(ctx),
    };

    Server::builder()
        .add_service(ClipsyncServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
