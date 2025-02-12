use std::net::IpAddr;
use tokio_util::codec::Framed;
use tun::{AsyncDevice, Configuration, TunPacketCodec};

pub struct TunInterface {
    pub config: Configuration,
    //pub framed: Framed<AsyncDevice, TunPacketCodec>, /*>>,*/
    pub device: AsyncDevice,
}

impl TunInterface {
    pub fn new(addr: IpAddr, dest: IpAddr) -> TunInterface {
        println!("ip:{}", addr);
        // create TUN device
        let mut config = Configuration::default();
        config
            .address(addr)
            .netmask((255, 255, 255, 0))
            //TODODODOOD .destination(dest)
            .mtu(1200)
            .layer(tun::Layer::L3)
            .up();
        #[cfg(target_os = "linux")]
        config.platform_config(|config| {
            // requiring root privilege to acquire complete functions
            config.ensure_root_privileges(true);
        });
        let dev = tun::create_as_async(&config).unwrap();
        //let framed = /*Arc::new(Mutex::new(*/dev.into_framed(); //));
        /*        let (send, recv) = dev.split();*/
        //TunInterface { config, framed }
        TunInterface {
            config,
            device: dev,
        }
    }
}
