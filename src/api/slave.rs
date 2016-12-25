use rosxmlrpc;
use rustc_serialize::{Decodable, Encodable};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use super::error::ServerError as Error;
use super::slavehandler::{add_publishers_to_subscription, SlaveHandler};
use tcpros::{self, Message, Publisher, PublisherStream, Subscriber};

pub struct Slave {
    name: String,
    server: rosxmlrpc::Server,
    publications: Arc<Mutex<HashMap<String, Publisher>>>,
    subscriptions: Arc<Mutex<HashMap<String, Subscriber>>>,
}

type SerdeResult<T> = Result<T, Error>;

impl Slave {
    pub fn new(master_uri: &str, server_uri: &str, name: &str) -> Result<Slave, Error> {
        let handler = SlaveHandler::new(master_uri, name);
        let pubs = handler.publications.clone();
        let subs = handler.subscriptions.clone();
        let server = rosxmlrpc::Server::new(server_uri, handler)?;
        Ok(Slave {
            name: String::from(name),
            server: server,
            publications: pubs,
            subscriptions: subs,
        })
    }

    pub fn uri(&self) -> &str {
        return &self.server.uri;
    }

    pub fn add_publishers_to_subscription<T>(&mut self,
                                             topic: &str,
                                             publishers: T)
                                             -> SerdeResult<()>
        where T: Iterator<Item = String>
    {
        add_publishers_to_subscription(&mut self.subscriptions.lock().unwrap(),
                                       &self.name,
                                       topic,
                                       publishers)
    }

    pub fn add_publication<T>(&mut self,
                              hostname: &str,
                              topic: &str)
                              -> Result<PublisherStream<T>, tcpros::Error>
        where T: Message + Encodable
    {
        use std::collections::hash_map::Entry;
        match self.publications.lock().unwrap().entry(String::from(topic)) {
            Entry::Occupied(publisher_entry) => publisher_entry.get().stream(),
            Entry::Vacant(entry) => {
                let publisher = Publisher::new::<T, _>(format!("{}:0", hostname).as_str(), topic)?;
                entry.insert(publisher).stream()
            }
        }
    }

    pub fn remove_publication(&mut self, topic: &str) {
        self.publications.lock().unwrap().remove(topic);
    }

    pub fn add_subscription<T, F>(&mut self, topic: &str, callback: F) -> Result<(), Error>
        where T: Message + Decodable,
              F: Fn(T) -> () + Send + 'static
    {
        use std::collections::hash_map::Entry;
        match self.subscriptions.lock().unwrap().entry(String::from(topic)) {
            Entry::Occupied(..) => {
                error!("Duplicate subscription to topic '{}' attempted", topic);
                Err(Error::Critical(String::from("Could not add duplicate subscription to topic")))
            }
            Entry::Vacant(entry) => {
                let subscriber = Subscriber::new::<T, F>(&self.name, topic, callback);
                entry.insert(subscriber);
                Ok(())
            }
        }
    }

    pub fn remove_subscription(&mut self, topic: &str) {
        self.subscriptions.lock().unwrap().remove(topic);
    }
}
