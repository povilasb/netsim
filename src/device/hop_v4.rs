use priv_prelude::*;

/// Connects two `Ipv4Plug`s and adds a hop between them. This causes the TTL value of all packets
/// to be decremented while travelling along the connection (and dropped if the TLL reaches zero).
pub struct HopV4 {
    plug_a: Ipv4Plug,
    plug_b: Ipv4Plug,
}

impl HopV4 {
    /// Create a new hop by connecting the two given plugs.
    pub fn new(
        plug_a: Ipv4Plug,
        plug_b: Ipv4Plug,
    ) -> HopV4 {
        HopV4 {
            plug_a,
            plug_b,
        }
    }

    /// Create a new hop by connecting the two given plugs. Spawn the `Hopv4` directly onto the
    /// tokio event loop.
    pub fn spawn(
        handle: &Handle,
        plug_a: Ipv4Plug,
        plug_b: Ipv4Plug,
    ) {
        let hop_v4 = HopV4::new(plug_a, plug_b);
        handle.spawn(hop_v4.infallible());
    }
}

impl Future for HopV4 {
    type Item = ();
    type Error = Void;

    fn poll(&mut self) -> Result<Async<()>, Void> {
        let a_unplugged = loop {
            match self.plug_a.rx.poll().void_unwrap() {
                Async::NotReady => break false,
                Async::Ready(None) => break true,
                Async::Ready(Some(mut packet)) => {
                    let next_ttl = match packet.ttl().checked_sub(1) {
                        Some(ttl) => ttl,
                        None => {
                            info!(
                                "hop dropping packet due to expired ttl: {:?}",
                                packet
                            );
                            continue;
                        },
                    };
                    let fields = packet.fields();
                    packet.set_fields(Ipv4Fields {
                        ttl: next_ttl,
                        .. fields
                    });
                    let _ = self.plug_b.tx.unbounded_send(packet);
                },
            }
        };

        let b_unplugged = loop {
            match self.plug_b.rx.poll().void_unwrap() {
                Async::NotReady => break false,
                Async::Ready(None) => break true,
                Async::Ready(Some(mut packet)) => {
                    let next_ttl = match packet.ttl().checked_sub(1) {
                        Some(ttl) => ttl,
                        None => {
                            info!(
                                "hop dropping packet due to expired ttl: {:?}",
                                packet
                            );
                            continue;
                        },
                    };
                    let fields = packet.fields();
                    packet.set_fields(Ipv4Fields {
                        ttl: next_ttl,
                        .. fields
                    });
                    let _ = self.plug_a.tx.unbounded_send(packet);
                },
            }
        };

        if a_unplugged && b_unplugged {
            return Ok(Async::Ready(()));
        }

        Ok(Async::NotReady)
    }
}

