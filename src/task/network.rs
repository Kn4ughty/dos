use core::task::{Context, Poll};
use futures_util::{Stream, StreamExt};

use crate::pci::rtl8139::RTL8139;
use crate::println;

pub struct NetworkStream {
    rtl: RTL8139,
}

impl Stream for NetworkStream {
    type Item = ();

    fn poll_next(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let _res = self.rtl.receive_packet();
        // println!("poll res: {:?}", res);

        Poll::Pending
    }
}

pub async fn get_packet(rtl: RTL8139) {
    let mut nns = NetworkStream { rtl };

    while let Some(_) = nns.next().await {
        println!("omg gt some:");
    }
}
