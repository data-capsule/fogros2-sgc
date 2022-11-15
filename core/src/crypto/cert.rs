extern crate openssl;
use openssl::stack::Stack;
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName,
    SubjectKeyIdentifier,
};
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::{X509Name, X509Req, X509StoreContext, X509VerifyResult, X509};
use regex::Regex;

pub fn debug_cert(cert: X509) {
    // print out the certificate
    let debugged = format!("{:#?}", cert);
    print!("{}", debugged);
    print!("{:#?}", cert.subject_name());
}

pub fn extract_gdp_name_from_subject(subject: &str) -> Option<&str> {
    // Currently assume only gdp name is the only field
    let reg = Regex::new(r#""(\w+)""#).unwrap();
    if (reg.is_match(&subject)) {
        let mat = reg.find(&subject).unwrap();
        Some(&subject[mat.start()..mat.end()])
    } else {
        None
    }
}

pub fn test_cert() {
    let cert_str = include_bytes!("../../resources/router.pem");
    let cert = X509::from_pem(cert_str).unwrap();

    let subject = format!("{:?}", cert.subject_name());

    print!("{:?}", extract_gdp_name_from_subject(&subject));
    let cert = include_bytes!("../../resources/router.pem");
    let cert = X509::from_pem(cert).unwrap();
    let ca = include_bytes!("../../resources/ca-root.pem");
    let ca = X509::from_pem(ca).unwrap();
    let chain = Stack::new().unwrap();

    let mut store_bldr = X509StoreBuilder::new().unwrap();
    store_bldr.add_cert(ca).unwrap();
    let store = store_bldr.build();

    let mut context = X509StoreContext::new().unwrap();
    assert!(context
        .init(&store, &cert, &chain, |c| c.verify_cert())
        .unwrap());
    assert!(context
        .init(&store, &cert, &chain, |c| c.verify_cert())
        .unwrap());
}
