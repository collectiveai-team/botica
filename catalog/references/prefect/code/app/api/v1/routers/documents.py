"""The endpoints of documents: the API starts flow runs and imports no flow."""

from fastapi import APIRouter, status

from app.api.v1.schemas.requests.documents import SummarizeRequest
from app.api.v1.schemas.responses.documents import SummarizeResponse
from app.core.documents import Document
from app.runs import start_summaries

router = APIRouter(prefix="/documents", tags=["documents"])


@router.post("/summaries", status_code=status.HTTP_202_ACCEPTED)
async def summarize(payload: SummarizeRequest) -> SummarizeResponse:
    """Start a flow run that summarizes the documents, and return its id."""
    documents = [Document(id=item.id, text=item.text) for item in payload.documents]
    return SummarizeResponse(flow_run_id=await start_summaries(documents))
