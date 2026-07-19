use iroh_gossip::net::Gossip;

pub struct GossipService {
    gossip: Gossip,
}

impl GossipService {
    pub fn new(gossip: &Gossip) -> Self {
        Self { gossip: gossip.clone() }
    }

    pub fn inner(&self) -> &Gossip {
        &self.gossip
    }
}
