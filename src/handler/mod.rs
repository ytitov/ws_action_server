use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, Bytes, BytesMut};
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use ws::{Handler, Message, Result as WSResult, Sender};

use crate::service::{ActionRequestTx, Service};
use json_action::{action::Action, error::ActionError};

//use futures::Sink; // trait, don't remove

pub struct Client {
    pub service: Arc<Mutex<Service>>,
    pub out: Sender,
    pub client_id: usize,
    pub tx_to_service: ActionRequestTx,
}

impl Client {
    /// the format of the binary message is such that the first 32 bits
    /// are four bytes, encoded as big endian as a u32.  This gives
    /// the size of the json string which follows it, then the rest of
    /// the message is arbitrarily in bytes, explained by the json string
    /// which is still to be determined
    fn decode_binary(&self, msg: Message) -> Result<(Action, Bytes), String> {
        let data_vec = msg.into_data();
        let total_size = data_vec.len() - 4;
        let mut rdr = Cursor::new(data_vec);

        // this is the size of the json string in bytes
        // the json should contain an action, and describe the binary
        // content
        //if let json_str_len = match rdr.read_u32::<BigEndian>().unwrap();
        if let Ok(json_str_len) = rdr.read_u32::<BigEndian>() {
            let mut buf: Vec<u8> = Vec::with_capacity(json_str_len as usize);
            for _i in 0..json_str_len {
                if let Ok(val) = rdr.read_u8() {
                    buf.push(val);
                } else {
                    return Err("Could not read the json string".to_owned());
                }
                match rdr.read_u8() {
                    Ok(val) => buf.push(val),
                    Err(e) => {
                        return Err(format!("Could not read the json string; Error: {}; trying to decode: {:?}; total_size: {}: json_str_len: {}; binary buf size: {}", 
                                e.to_string(), &buf,
                                total_size,
                                json_str_len,
                                total_size - json_str_len as usize,
                        ));
                    },
                }
            }

            //let mut buf_bytes: Vec<u8> = Vec::with_capacity(total_size - json_str_len as usize);
            let buf_size = total_size - json_str_len as usize;
            let mut buf_bytes = BytesMut::with_capacity(buf_size.clone());
            for _i in 0..buf_size {
                if let Ok(val) = rdr.read_u8() {
                    buf_bytes.put(val);
                } else {
                    return Err("Could not read the binary data".to_owned());
                }
            }

            match serde_json::from_slice::<Action>(buf.as_slice()) {
                Ok(msg_req) => Ok((msg_req, buf_bytes.freeze())),
                Err(e) => Err(format!("serde_json: {}, trying to decode: {:?}; total_size: {}: json_str_len: {}; binary buf size: {}", 
                        e.to_string(), &buf,
                        total_size,
                        json_str_len,
                        buf_size,
                        )),
            }
        } else {
            return Err("Could not get the size of the json string in message".to_owned());
        }
    }

    //fn send (&self, s: (usize, MessageRequest, Option<Bytes>, BytesActionTx, ws::Sender)) {
    // TODO: handle this error and disconnect client
    fn send(&mut self, s: (usize, Action, Option<Bytes>, ws::Sender)) {
        match self.tx_to_service.unbounded_send(s) {
        //match self.tx_to_service.clone().wait().send(s) {
            Ok(_) => (),
            Err(e) => {
                println!("handler/mod.rs ERROR: {:?}", e);
                self.on_error(ws::Error::new(
                    ws::ErrorKind::Internal,
                    "There was an error sending the response back to client",
                ));
            }
        };
    }
}

use serde_json::to_string as json_to_string;
impl Handler for Client {
    fn on_message(&mut self, msg: Message) -> WSResult<()> {
        // if binary, Some(bytes) are sent else None is sent and send_bytes channel doesn't need to be used
        // though some actions want to make use of it
        if msg.is_binary() {
            match self.decode_binary(msg) {
                Ok((msg_req, buf_bytes)) => {
                    self.send((self.client_id, msg_req, Some(buf_bytes), self.out.clone()));
                    Ok(())
                }
                Err(e) => self.out.send(Message::text(
                    json_to_string(&Action::server_err(ActionError::new(
                        "BinaryToAction",
                        e.as_ref(),
                    )))
                    .unwrap(),
                )),
            }
        } else {
            //match serde_json::from_slice::<Action>(msg.into_text().unwrap().as_ref()) {
            match serde_json::from_slice::<Action>(msg.into_text().unwrap().as_ref()) {
                Ok(action) => {
                    self.send((self.client_id, action, None, self.out.clone()));
                    Ok(())
                }
                Err(e) => self.out.send(Message::text(
                    json_to_string(&Action::server_err(ActionError::new(
                        "StringToAction",
                        &e.to_string(),
                    )))
                    .unwrap(),
                )),
            }
        }
    }

    fn on_open(&mut self, _shake: ws::Handshake) -> WSResult<()> {
        //println!("on_open {:?}", _shake);
        Ok(())
    }

    fn on_close(&mut self, _code: ws::CloseCode, _reason: &str) {
        //println!("disconnecting client {}", self.client_id);
        self.service.lock().unwrap().remove_client(self.client_id);
    }

    fn on_error(&mut self, err: ws::Error) {
        println!("on_error {:?}", err);
        self.service.lock().unwrap().remove_client(self.client_id);
        match self.out.close(ws::CloseCode::Error) {
            Ok(_) => {
                println!("socket closed");
            }
            Err(_) => {
                println!("error closing socket");
            }
        };

        //self.inner.on_error(err);
    }
}
