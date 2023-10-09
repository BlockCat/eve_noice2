use super::StartActor;
use actix::{Actor, Context, Recipient};
use tokio::task::JoinHandle;
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Debug)]
pub struct UpdateScheduler(String, Vec<Recipient<StartActor>>, Option<JoinHandle<()>>);

impl UpdateScheduler {
    pub fn new(cron: String, recipients: Vec<Recipient<StartActor>>) -> Self {
        Self(cron, recipients, None)
    }
}

impl Actor for UpdateScheduler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let cron = self.0.clone();
        log::debug!("Started UpdateScheduler with cron: {}", cron);
        let recipients = self.1.clone();
        self.2 = Some(tokio::spawn(async move {
            let scheduler = JobScheduler::new().await.unwrap();
            let cron = cron.clone();
            let recipients = recipients.clone();
            let job = Job::new_cron_job(cron.as_str(), move |_, _| {
                for recipient in recipients.iter() {
                    match recipient.do_send(StartActor) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
            })
            .unwrap();
            scheduler.add(job).await.unwrap();

            match scheduler.start().await {
                Ok(_) => {}
                Err(_) => {}
            }
        }));
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> actix::Running {
        if let Some(handle) = self.2.take() {
            handle.abort();
        }
        actix::Running::Stop
    }
}
