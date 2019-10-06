use error_chain::error_chain;

pub mod peer {
    use error_chain::error_chain;

    error_chain! {
        errors {
            ConnectFailed {}
            HasNoPeerAddr {}
            HasNoLocalAddr {}
            ReadMessageFailed {}
            Disconnected {}
        }
    }
}

pub mod message {
    use error_chain::error_chain;

    error_chain! {
        errors {
            IoError {}
            InvalidChecksum {}
            WrongMagic(magic: Vec<u8>) {
                description("Wrong message magic")
                display("Wrong message: {}", hex::encode(&magic))
            }
        }
    }
}

error_chain! {
    links {
        Peer(peer::Error, peer::ErrorKind);
        Message(message::Error, message::ErrorKind);
    }

    errors {
        ChannelError {}
    }
}
