use crate::backend::{
    database::{Database, PrimaryKey},
    document::Document,
};
use rocket::{
    http::{ContentType, Header},
    response::{self, Responder},
    Request, Response,
};

#[derive(Debug, Clone)]
pub struct PdfOutput(Vec<u8>);

impl PdfOutput {
    pub fn new(
        database: &Database,
        document_id: PrimaryKey<Document>,
    ) -> Result<Self, crate::Error> {
        Ok(PdfOutput(Document::load_into_memory(
            database,
            document_id,
        )?))
    }
}

impl<'r> Responder<'r, 'r> for PdfOutput {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        Response::build()
            .header(ContentType::PDF)
            .header(Header::new("Content-Disposition", "inline"))
            .sized_body(self.0.len(), std::io::Cursor::new(self.0))
            .ok()
    }
}
