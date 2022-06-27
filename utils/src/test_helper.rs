use pink_extension::chain_extension::mock;

pub fn mock_crypto() {
    mock::mock_derive_sr25519_key(|_| {
        hex::decode("78003ee90ff2544789399de83c60fa50b3b24ca86c7512d0680f64119207c80ab240b41344968b3e3a71a02c0e8b454658e00e9310f443935ecadbdd1674c683").unwrap()
    });
    mock::mock_get_public_key(|_| {
        hex::decode("ce786c340288b79a951c68f87da821d6c69abd1899dff695bda95e03f9c0b012").unwrap()
    });
    mock::mock_sign(|_| b"mock-signature".to_vec());
    mock::mock_verify(|_| true);
}

pub fn mock_log() {
    let levels = ["Error", "Warn", "Info", "Debug", "Trace"];
    mock::mock_log(move |level, msg| println!("📜 [{}] {}", levels[level as usize], msg));
}

pub fn mock_all() {
    mock_crypto();
    mock_log();
}
