fn main() {
    embuild::espidf::sysenv::output();

    // Enable WebSocket client configuration flags manually
    // These are required for esp-idf-svc WebSocket client to be available
    println!("cargo:rustc-cfg=esp_idf_comp_tcp_transport_enabled");
    println!("cargo:rustc-cfg=esp_idf_comp_esp_tls_enabled");

    // Since we're using ESP-IDF 5.x, enable the external component flag
    println!("cargo:rustc-cfg=esp_idf_comp_espressif__esp_websocket_client_enabled");

    // Also set the alloc feature flag that esp-idf-svc expects
    println!("cargo:rustc-cfg=feature=\"alloc\"");
}
