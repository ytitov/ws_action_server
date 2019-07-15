use bytes::Bytes;
use futures::sync::mpsc;
use json_action::action::Action;
use serde_json;
use std::collections::HashMap;
use ws;
/// Shorthand for the transmit half of the message channel.
pub type ActionRequestTx = mpsc::UnboundedSender<(usize, Action, Option<Bytes>, ws::Sender)>;
pub type ActionRequestRx = mpsc::UnboundedReceiver<(usize, Action, Option<Bytes>, ws::Sender)>;
pub type BytesActionTx = mpsc::UnboundedSender<(usize, Action, Bytes)>;
pub type BytesActionRx = mpsc::UnboundedReceiver<(usize, Action, Bytes)>;
pub type ActionTx = mpsc::UnboundedSender<(usize, Action)>;
pub type ActionRx = mpsc::UnboundedReceiver<(usize, Action)>;

pub struct Service {
    ids: HashMap<usize, ws::Sender>,
}

impl Service {
    pub fn new() -> Self {
        Service {
            ids: HashMap::with_capacity(10000),
        }
    }

    fn get_new_id(&mut self, sender: ws::Sender) -> usize {
        for i in 1..std::usize::MAX {
            if self.ids.contains_key(&i) == false {
                self.ids.insert(i, sender);
                return i;
            }
        }
        0
    }

    fn remove_id(&mut self, i: usize) -> bool {
        if self.ids.contains_key(&i) == true {
            self.ids.remove(&i);
            println!("num_clients: {}", self.ids.len());
            true
        } else {
            false
        }
    }

    //pub fn add_client(&self, id: usize) {
    pub fn add_client(&mut self, _sender: ws::Sender) -> Result<usize, ()> {
        let id = self.get_new_id(_sender);
        if id == 0 {
            Err(())
        } else {
            println!("added client with id {}", id);
            Ok(id)
        }
    }

    //pub fn remove_client(&self, id: usize) {
    pub fn remove_client(&mut self, id: usize) {
        println!("remove client called {:?}", id);
        self.remove_id(id);
    }

    pub fn socket_reply(&mut self, res: (usize, Action)) {
        let (client_id, msg) = res;
        //self.ids.get(&client_id).unwrap().send(ws::Message::text(serde_json::to_string(&msg).unwrap()));
        if let Some(sender) = self.ids.get(&client_id) {
            match sender.send(ws::Message::text(serde_json::to_string(&msg).unwrap())) {
                Err(_) => {
                    println!("some error happened while trhingn to send, removing client");
                    self.remove_client(client_id);
                }
                _ => println!("replied to client_id {:?}", client_id),
            };
        } else {
            println!("no sender found, removing client");
            self.remove_client(client_id);
        }
    }

    pub fn get_sender(&self, client_id: usize) -> Option<ws::Sender> {
        if let Some(sender) = self.ids.get(&client_id) {
            //drop(&self);
            Some(sender.clone())
        } else {
            println!("no sender found, removing client");
            //drop(&self);
            None
        }
    }
}
