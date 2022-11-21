use std::net::SocketAddr;

use crate::gdp_proto::globaldataplane_client::GlobaldataplaneClient;
use crate::gdp_proto::globaldataplane_server::{Globaldataplane, GlobaldataplaneServer};
use crate::gdp_proto::{GdpPacket, GdpResponse, GdpUpdate};
use futures::future;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tonic::transport::Error;
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug)]
pub struct GDPService {
    // receive GDP packet proto from somewhere else 
    pub scheduler_tx: Sender<GdpPacket>,
    // the control data 
    pub status_tx: Sender<GdpUpdate>,
}

#[tonic::async_trait]
impl Globaldataplane for GDPService {
    async fn gdp_forward(
        &self, request: Request<GdpPacket>,
    ) -> Result<Response<GdpResponse>, Status> {
        println!("Got a request: {:?}", request);
        let request_body = request.into_inner();

        self.scheduler_tx.send(request_body).await;

        // TODO: a more meaningful ack? 
        let forward_response = "ACK";
        println!("{}", forward_response);

        let reply = GdpResponse {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("{}", forward_response).into(),
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }

    async fn gdp_update(
        &self, update_request: Request<GdpUpdate>,
    ) -> Result<Response<GdpResponse>, Status> {
        println!("Got an update: {:?}", update_request);
        // let mut status = Pslstatus {
        //     sender_addr: update_request.remote_addr().unwrap().to_string(),
        //     ..Default::default()
        // };

        let update = update_request.into_inner();

        self.status_tx.send(update).await;

        let reply = GdpResponse {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("{}", "ACK").into(),
        };
        Ok(Response::new(reply))
    }
}


pub async fn forward_execute_request(msg: GdpPacket, addr: String) {
    // dispatch the task async-ly
    tokio::spawn(async move {
        let mut client = GlobaldataplaneClient::connect(addr)
            .await
            .expect("Client connect failure");
        let request = tonic::Request::new(msg);
        let response = client.gdp_forward(request).await.expect("grpc failure");
        println!("RESPONSE={:?}", response);
    });
}

pub async fn forward_status_async(msg: GdpUpdate, addr: String) {
    // dispatch the task async-ly
    tokio::spawn(async move {
        let mut client = GlobaldataplaneClient::connect(addr)
            .await
            .expect("Client connect failure");
        let request = tonic::Request::new(msg);
        let response = client.gdp_update(request).await.expect("grpc failure");
        println!("RESPONSE={:?}", response);
    });
}

#[tokio::main]
pub async fn forward_status(msg: GdpUpdate, addr: String) {
    let handle = tokio::spawn(async move {
        let mut client = GlobaldataplaneClient::connect(addr)
            .await
            .expect("Client connect failure");
        let request = tonic::Request::new(msg);
        let response = client.gdp_update(request).await.expect("grpc failure");
        println!("RESPONSE={:?}", response);
    });
    future::join_all([handle]).await;
}
