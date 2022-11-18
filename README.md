<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**

- [GDP Router](#gdp-router)
    - [How to run](#how-to-run)
    - [testcases](#testcases)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# GDP Router 

### How to run 
```
cargo build && sudo ./target/debug/gdp-router router
```


### testcases 

We can come up with the following test case: 
a router, a dtls client and a tcp client. We want to route tcp client's message
to dtls client. 
```bash
# (terminal A) run router
$ cargo run router

# (terminal B) run dtls client 
$ cargo run client

# (terminal C) run tcp client
$ nc localhost 9997
```

Then we can use the following sample test cases
```
# (dtls client) advertise itself with name 1
ADV,1
FWD,1,000 // this sends itself a message

# (tcp client) send message to name 1
FWD,1,111
FWD,1,222
FWD,1,333
FWD,1,444
```
We should expect messages appearing in dtls client's terminal.

