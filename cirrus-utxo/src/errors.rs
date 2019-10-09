use error_chain::error_chain;

error_chain! {
    errors {
        ConnectionError {}
        ChannelError {}
    }
}
