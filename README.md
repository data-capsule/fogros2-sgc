# GDP Router 

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**

- [How to build](#how-to-build)
  - [(Optional) Install grpcurl](#optional-install-grpcurl)
  - [grpc](#grpc)
- [TODOs](#todos)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->


### How to build 

```
cargo build
```

#### (Optional) Install grpcurl
`grpcurl` helps with testing the grpc interface. Run 
```
curl -sSL "https://github.com/fullstorydev/grpcurl/releases/download/v1.8.7/grpcurl_1.8.7_linux_x86_64.tar.gz" | sudo tar -xz -C /usr/local/bin
```
to install. 

#### grpc 
```
grpcurl -plaintext -import-path proto -proto gdp.proto -d '{"sender": "sender", "receiver": "receiver", "action": 1, "payload":"RldELDEsMDAw"}' '[::]:50001' gdp.Globaldataplane/gdp_forward
```
ceveat: convert payload to byte64 encoding (e.g. `RldELDEsMDAw` is `FWD,1,000`. A useful tool can be found [here](https://www.base64encode.org/))

### TODOs
- [x] tcp test interface
- [x] dlts 
- [x] connection RIB 
- [ ] an actual future based RIB 
- [ ] zero copy multicast (can we adopt the design from pnet version?)
- [ ] grpc stream
- [ ] use gdp protocol (protobuf or just bytes?) 
- [ ] use name certificates instead of pseudo names  

minor 
- [ ] use app_config to config the ports and addresses 
- [ ] enhance error handling (e.g. connection is closed, packet wrong format)