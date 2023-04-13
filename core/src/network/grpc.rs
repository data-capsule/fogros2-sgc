use crate::gdp_proto::globaldataplane_client::GlobaldataplaneClient;
use crate::gdp_proto::globaldataplane_server::Globaldataplane;
use crate::gdp_proto::{GdpPacket, GdpResponse, GdpUpdate};
use crate::structs::GDPPacket;
use futures::future;
use tokio::sync::mpsc::Sender;

use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct GDPService {
    // send GDP packet to connection RIB
    // NOTE: this is not the gdp packet in protobuf
    // it is the gdp packet in struct
    // (shall we rename it?)
    pub fib_tx: Sender<GDPPacket>,
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

        let packet = populate_gdp_struct_from_proto(request_body);

        self.fib_tx.send(packet).await.expect("send packet failed");

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

        self.status_tx
            .send(update)
            .await
            .expect("send update failed");

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

pub async fn forward_status(msg: GdpUpdate, addr: String) {
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

/// synchronously send update to an address
/// typically used when the switch is initialized
#[tokio::main]
pub async fn forward_status_sync(msg: GdpUpdate, addr: String) {
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
