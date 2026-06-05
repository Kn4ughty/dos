use core::task::{Context, Poll};
use futures_util::{Stream, StreamExt};

use crate::pci::rtl8139::{self, RTL8139};
use crate::println;

pub struct NetworkStream {
    // rtl: RTL8139,
}

impl Stream for NetworkStream {
    type Item = ();

    fn poll_next(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        println!("Getting packet");
        let _res = rtl8139::RTL
            .get()
            .expect("RTL should already be init")
            .lock()
            .receive_packet();
        // println!("poll res: {:?}", res);

        Poll::Pending
    }
}

pub async fn get_packet() {
    let mut nns = NetworkStream {};

    while let Some(_) = nns.next().await {
        println!("omg gt some:");
    }
}
