use at_protocol::*;

fn main() {
    let mut interface = Interface::new("/dev/ttyUSB0").unwrap();

    let test_success = interface.send(command::Test).unwrap();
    assert!(test_success);
    println!("Status: {}", if test_success { "OK" } else { "Error" });

    let version = interface.send(command::GetVersion).unwrap();
    println!("Version {:?}", version);

    let mode = interface.send(command::GetWifiMode).unwrap();
    println!("Wifi mode is {:?}", mode);

    let connected_ap = interface.send(command::GetConnectedAp).unwrap();

    if let Some(ap) = connected_ap {
        println!("Connected to {:?}", ap);
        println!("Disconnecting...");
        interface.send(command::DisconnectFromAp).unwrap();
        println!("Validating that we're disconnected..");

        assert!(interface.send(command::GetConnectedAp).unwrap().is_none());
        println!("Ok!");
    }

    interface
        .send(command::ConnectToAp {
            ssid: "",
            password: "",
        })
        .unwrap();

    println!("Connected to AP");
}
