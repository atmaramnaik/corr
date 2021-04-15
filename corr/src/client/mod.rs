use corr_lib::core::runtime::{Client};
use corr_lib::core::runtime::Context as CorrContext;

use corr_lib::core::proto::{Input, Output};
use async_trait::async_trait;
pub struct CliDriver;
use flate2::read::GzDecoder;
use std::fs::{File, create_dir_all};
use tar::Archive;
use std::path::Path;
use corr_lib::journey::{Journey, Executable};
use std::sync::Arc;
use futures::lock::Mutex;


use tokio::sync::{mpsc};
use tokio::sync::mpsc::{Receiver, Sender};



use corr_lib::parser::Parsable;

use tokio::io::{Stdin, BufReader, AsyncBufReadExt, Lines};



impl CliDriver{
    pub async fn run(target:String,journey:String){
        let jp=unpack(target).unwrap();
        Self::run_journey_in(jp,journey).await;
    }
    pub async fn run_journey_in(jp:String,journey:String){
        let j = if journey.eq("<default>"){
            let mut dir = tokio::fs::read_dir(format!("{}/src",jp)).await.unwrap();
            let entry = dir.next_entry().await.unwrap().unwrap();
            let contents = tokio::fs::read_to_string(entry.path()).await.unwrap();
            contents
        } else {
            let contents = tokio::fs::read_to_string(format!("{}/src/{}.journey",jp,journey)).await.unwrap();
            contents
        };

        let (_,jrn) = Journey::parser(j.as_str()).unwrap();//Self::get_journey(jp,journey);
        let mut terminal = Terminal::new();
        let context = CorrContext::new(Arc::new(Mutex::new(terminal.get_if())));
        tokio::spawn(async move {
            start(jrn,context).await;
        });
        terminal.start().await;

    }
}
pub async fn start(journey:Journey,context:CorrContext) {
    println!("Starting Journey");
    journey.execute(&context).await;
    context.user.lock().await.send(Output::new_done("Done Executing Journey".to_string())).await;
}
fn unpack(target:String) -> Result<String, std::io::Error> {
    let tc = target.clone();
    let path = Path::new(tc.as_str());
    let name = path.file_stem();
    let tar_gz = File::open(target)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    create_dir_all("./target");
    let jp = format!("./target/{}",name.unwrap().to_str().unwrap());
    archive.unpack(jp.clone())?;
    Ok(jp)
}
#[derive(Debug, Clone)]
pub enum Message{
    Input(Input),
    Output(Output)
}
pub struct Terminal{
    tx:Sender<Message>,
    rx:Receiver<Message>,
    reader:Lines<BufReader<Stdin>>
}
impl Terminal{
    pub fn new()->Self{
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let (tx,rx) = mpsc::channel(100);
        return Self {
            tx,
            rx,
            reader:reader.lines(),
        }
    }
    pub async fn start(&mut self){
        loop {
            if let Some(message) = self.rx.recv().await {
                if self.on_message(message).await {
                    return
                }
            }
        }
    }
    pub async fn on_message(&mut self,message:Message)->bool{
        match message {
            Message::Output(Output::KnowThat(kt))=>{
                println!("{}",kt.message);
                false
            },
            Message::Output(Output::TellMe(tm))=>{
                println!("Please Enter value for {} of type {:?}",tm.name,tm.data_type);
                let nl=self.reader.next_line().await.unwrap().unwrap();
                self.tx.send(Message::Input(Input::new_continue(tm.name.clone(),nl,tm.data_type.clone()))).await;
                false
            },
            Message::Output(Output::Done(dom))=>{
                println!("{}",dom.message);
                true
            },
            _=>{unimplemented!()}
        }
    }
    pub fn get_if(&mut self)->CliInterface{
        let (tx,rx) = mpsc::channel(100);
        let (tx_s,rx_s) = mpsc::channel(100);
        self.rx = rx;
        self.tx = tx_s;
        return CliInterface{
            tx,
            rx:rx_s
        }
    }
}
pub struct CliInterface{
    tx:Sender<Message>,
    rx:Receiver<Message>
}
#[async_trait]
impl Client for CliInterface {
    async fn send(&self,output:Output){
        if let Err(_err) = &self.tx.send(Message::Output(output)).await {
            println!("Some Error")
        }
    }
    async fn get_message(&mut self)->Input{
        loop {
            if let Some(Message::Input(ip)) = self.rx.recv().await {
                return ip;
            }
        }
    }
}


