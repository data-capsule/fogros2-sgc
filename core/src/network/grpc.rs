use std::net::SocketAddr;

use crate::executor::loader::wasm_execute;
use crate::psl_proto::paranoidlambda_client::ParanoidlambdaClient;
use crate::psl_proto::paranoidlambda_server::{Paranoidlambda, ParanoidlambdaServer};
use crate::psl_proto::{ExecuteReply, ExecuteRequest, StatusUpdate};
use crate::structs::Pslstatus;
use futures::future;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tonic::transport::Error;
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug)]
pub struct PSLService {
    pub scheduler_tx: Sender<ExecuteRequest>,
    pub status_tx: Sender<Pslstatus>,
}

#[tonic::async_trait]
impl Paranoidlambda for PSLService {
    async fn psl_execute(
        &self, request: Request<ExecuteRequest>,
    ) -> Result<Response<ExecuteReply>, Status> {
        println!("Got a request: {:?}", request);
        let request_body = request.into_inner();

        self.scheduler_tx.send(request_body).await;

        // TODO: generate result from real handle
        let execution_result = "faked_handle_of_result";
        println!("{}", execution_result);

        let reply = ExecuteReply {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("{}", execution_result).into(),
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }

    async fn psl_update(
        &self, update_request: Request<StatusUpdate>,
    ) -> Result<Response<ExecuteReply>, Status> {
        println!("Got an update: {:?}", update_request);
        let mut status = Pslstatus {
            sender_addr: update_request.remote_addr().unwrap().to_string(),
            ..Default::default()
        };

        let update = update_request.into_inner();

        status.status = update.status;
        status.sender_name = update.sender; // TODO: need some tuning here

        self.status_tx.send(status).await;

        let reply = ExecuteReply {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("{}", "response").into(),
        };
        Ok(Response::new(reply))
    }
}


pub async fn forward_execute_request(msg: ExecuteRequest, addr: String) {
    // dispatch the task async-ly
    tokio::spawn(async move {
        let mut client = ParanoidlambdaClient::connect(addr)
            .await
            .expect("Client connect failure");
        let request = tonic::Request::new(msg);
        let response = client.psl_execute(request).await.expect("grpc failure");
        println!("RESPONSE={:?}", response);
    });
}

pub async fn forward_status_async(msg: StatusUpdate, addr: String) {
    // dispatch the task async-ly
    tokio::spawn(async move {
        let mut client = ParanoidlambdaClient::connect(addr)
            .await
            .expect("Client connect failure");
        let request = tonic::Request::new(msg);
        let response = client.psl_update(request).await.expect("grpc failure");
        println!("RESPONSE={:?}", response);
    });
}

#[tokio::main]
pub async fn forward_status(msg: StatusUpdate, addr: String) {
    let handle = tokio::spawn(async move {
        let mut client = ParanoidlambdaClient::connect(addr)
            .await
            .expect("Client connect failure");
        let request = tonic::Request::new(msg);
        let response = client.psl_update(request).await.expect("grpc failure");
        println!("RESPONSE={:?}", response);
    });
    future::join_all([handle]).await;
}
