#![allow(non_snake_case)] // turns off warnings for ParityClient

use super::*;

jsonrpc_client!(pub struct ParityClient {
    pub fn parity_enode(&mut self) -> RpcRequest<String>;
    pub fn parity_netPort(&mut self) -> RpcRequest<u16>;
    pub fn parity_netPeers(&mut self) -> RpcRequest<Peers>;
    pub fn parity_addReservedPeer(&mut self, enode: String) -> RpcRequest<bool>;
    pub fn parity_removeReservedPeer(&mut self, enode: String) -> RpcRequest<bool>;
    pub fn shh_post(&mut self, post: Post) -> RpcRequest<bool>;
    pub fn shh_newMessageFilter(&mut self, filter: MessageFilter) -> RpcRequest<Binary>;
    pub fn shh_getFilterMessages(&mut self, filter: Binary) -> RpcRequest<Vec<Message>>;
});

