
export CA_NAME=ca
export CERT_NAME=testrouter
export CERT_GDP_NAME=25442A472D4B6150645367556B58703273357638792F423F4528482B4D625165
1. Generate private key of CA 
```
openssl genrsa -out $CA_NAME-private.pem 2048
```

2. Generate public key of the root CA 
```
openssl rsa -in $CA_NAME-private.pem -out $CA_NAME-public.pem
```

3. create a certificate signing request 
```
openssl req -sha256 -new -key $CA_NAME-private.pem -out $CERT_NAME.csr -subj /O=$CERT_GDP_NAME
```
This specifies the file to read the private key from. 

4. sign the certificate 
```
openssl x509 -req -sha256 -days 365 -in $CERT_NAME.csr -signkey $CA_NAME-private.pem -out $CERT_NAME.pem
```

