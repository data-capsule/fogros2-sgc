
# configure environment variables
export CA_NAME=ca
export NUM_KEY=3

# remove the previous crypto material 
rm -r crypto 

# generate the crypto material
mkdir crypto 
cd crypto 

# generate the crypto material for the orderer
openssl genrsa -out $CA_NAME-private.pem 2048
openssl rsa -in $CA_NAME-private.pem -out $CA_NAME-public.pem
openssl req -x509 -sha256 -new -nodes -key $CA_NAME-private.pem -days 3650 -out $CA_NAME-root.pem -subj "/O=GDP Root CA"

echo "Generated the crypto key for the root CA"

export CERT_GDP_NAME='test_cert'
export CERT_NAME='test_cert'
echo "Generating the crypto keys for "$CERT_NAME
openssl genrsa -out $CERT_NAME-private.pem 2048
openssl rsa -in $CERT_NAME-private.pem -out $CERT_NAME-public.pem
openssl req -sha256 -new -key $CERT_NAME-private.pem -out $CERT_NAME.csr -subj /O=$CERT_GDP_NAME
openssl x509 -req -sha256 -days 365 -in $CERT_NAME.csr -CA $CA_NAME-root.pem -CAkey $CA_NAME-private.pem -CAcreateserial -out $CERT_NAME.pem
cat ca-root.pem 2>/dev/null >> $CERT_NAME.pem 

mkdir $CERT_NAME 
cp $CERT_NAME-private.pem $CERT_NAME/$CERT_NAME-private.pem
cp $CERT_NAME.pem $CERT_NAME/$CERT_NAME.pem
cp ca-root.pem $CERT_NAME/ca-root.pem
rm $CERT_NAME-private.pem $CERT_NAME-public.pem $CERT_NAME.csr $CERT_NAME.pem

for (( i=1; i<=$NUM_KEY; i++ ))
do 
    export CERT_GDP_NAME=`cat /dev/urandom 2>/dev/null | tr -cd 'a-f0-9' 2>/dev/null | head -c 32`
    export CERT_NAME=${CERT_GDP_NAME:0:5} 
    echo "Generating the crypto keys for "$CERT_NAME
    openssl genrsa -out $CERT_NAME-private.pem 2048
    openssl rsa -in $CERT_NAME-private.pem -out $CERT_NAME-public.pem
    openssl req -sha256 -new -key $CERT_NAME-private.pem -out $CERT_NAME.csr -subj /O=$CERT_GDP_NAME
    openssl x509 -req -sha256 -days 365 -in $CERT_NAME.csr -CA $CA_NAME-root.pem -CAkey $CA_NAME-private.pem -CAcreateserial -out $CERT_NAME.pem
    cat ca-root.pem 2>/dev/null >> $CERT_NAME.pem 

    mkdir $CERT_NAME 
    cp $CERT_NAME-private.pem $CERT_NAME/$CERT_NAME-private.pem
    cp $CERT_NAME.pem $CERT_NAME/$CERT_NAME.pem
    cp ca-root.pem $CERT_NAME/ca-root.pem
    rm $CERT_NAME-private.pem $CERT_NAME-public.pem $CERT_NAME.csr $CERT_NAME.pem
done 
