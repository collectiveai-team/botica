from fastapi import APIRouter

from myapp.api.schemas.requests.contract import CreateContractRequest

router = APIRouter()


@router.post("/contracts")
def create(body: CreateContractRequest):
    return body
