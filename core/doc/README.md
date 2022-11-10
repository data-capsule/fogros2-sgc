
### Root CA Certificate Generation 

0. export environemnt variables
```
export CA_NAME=ca
```

1. Generate private key of CA 
```
openssl genrsa -out $CA_NAME-private.pem 2048
```

2. Generate public key of the root CA 
```
openssl rsa -in $CA_NAME-private.pem -out $CA_NAME-public.pem
```

3. Generate root CA self-signed certificate
```
openssl req -x509 -sha256 -new -nodes -key $CA_NAME-private.pem -days 3650 -out $CA_NAME-root.pem -subj "/O=GDP Root CA"
```
(https://www.ibm.com/docs/en/runbook-automation?topic=certificate-generate-root-ca-key)


### Individual GDP Names Certificate

0. export environment variables 
```
export CERT_NAME=router
export CERT_GDP_NAME=25442A472D4B6150645367556B58703273357638792F423F4528482B4D625165
```

1. Generate private key of CA 
```
openssl genrsa -out $CERT_NAME-private.pem 2048
```

2. Generate public key of the root CA 
```
openssl rsa -in $CERT_NAME-private.pem -out $CERT_NAME-public.pem
```


3. create a certificate signing request 
```
openssl req -sha256 -new -key $CERT_NAME-private.pem -out $CERT_NAME.csr -subj /O=$CERT_GDP_NAME
```
This specifies the file to read the private key from. 

4. sign the certificate 
```
openssl x509 -req -sha256 -days 365 -in $CERT_NAME.csr -CA $CA_NAME-root.pem -CAkey $CA_NAME-private.pem -CAcreateserial -out $CERT_NAME.pem
```

5. Make it a certificate chain 
```
cat ca-root.pem >> router.pem 
```

### Current status 

```
 openssl x509 -text -in ca-root.pem | grep -E '(Subject|Issuer):'
        Issuer: O = GDP Root CA
        Subject: O = GDP Root CA

openssl x509 -text -in router.pem | grep -E '(Subject|Issuer):'
        Issuer: O = GDP Root CA
        Subject: O = 25442A472D4B6150645367556B58703273357638792F423F4528482B4D625165
```
However, 
```
openssl verify -verbose -CAfile ca-root.pem -untrusted router.pem 
```
hangs 
