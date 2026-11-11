from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from ...models.app_attestation_purpose import AppAttestationPurpose
from ...models.app_attestation_response_200 import AppAttestationResponse200
from ...models.error import Error
from ...types import UNSET, Unset
from typing import cast
from typing import Union


def _get_kwargs(
    app_path: str,
    *,
    nonce: str,
    app_query: Union[Unset, str] = UNSET,
    app_hash: Union[Unset, str] = UNSET,
    purpose: Union[Unset, AppAttestationPurpose] = UNSET,
) -> dict[str, Any]:

    params: dict[str, Any] = {}

    params["nonce"] = nonce

    params["app"] = app_query

    params["appHash"] = app_hash

    json_purpose: Union[Unset, str] = UNSET
    if not isinstance(purpose, Unset):
        json_purpose = purpose.value

    params["purpose"] = json_purpose

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/apps/{app_path}/attestation".format(
            app_path=app_path,
        ),
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[AppAttestationResponse200, Error]]:
    if response.status_code == 200:
        response_200 = AppAttestationResponse200.from_dict(response.json())

        return response_200

    if response.status_code == 401:
        response_401 = Error.from_dict(response.json())

        return response_401

    if response.status_code == 421:
        response_421 = Error.from_dict(response.json())

        return response_421

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[AppAttestationResponse200, Error]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    app_path: str,
    *,
    client: AuthenticatedClient,
    nonce: str,
    app_query: Union[Unset, str] = UNSET,
    app_hash: Union[Unset, str] = UNSET,
    purpose: Union[Unset, AppAttestationPurpose] = UNSET,
) -> Response[Union[AppAttestationResponse200, Error]]:
    """Every replica's receipt-key attestation

     Each replica's `/attestation?purpose=receipts` for `nonce`. It reads the query `/attestation` does,
    but only `nonce`: `app`, `appHash` and `purpose` are ignored here.

    **Who:** whoever the app's access admits (a public app: anyone).

    Args:
        app_path (str):
        nonce (str):
        app_query (Union[Unset, str]):
        app_hash (Union[Unset, str]):
        purpose (Union[Unset, AppAttestationPurpose]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[AppAttestationResponse200, Error]]
    """

    kwargs = _get_kwargs(
        app_path=app_path,
        nonce=nonce,
        app_query=app_query,
        app_hash=app_hash,
        purpose=purpose,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    app_path: str,
    *,
    client: AuthenticatedClient,
    nonce: str,
    app_query: Union[Unset, str] = UNSET,
    app_hash: Union[Unset, str] = UNSET,
    purpose: Union[Unset, AppAttestationPurpose] = UNSET,
) -> Optional[Union[AppAttestationResponse200, Error]]:
    """Every replica's receipt-key attestation

     Each replica's `/attestation?purpose=receipts` for `nonce`. It reads the query `/attestation` does,
    but only `nonce`: `app`, `appHash` and `purpose` are ignored here.

    **Who:** whoever the app's access admits (a public app: anyone).

    Args:
        app_path (str):
        nonce (str):
        app_query (Union[Unset, str]):
        app_hash (Union[Unset, str]):
        purpose (Union[Unset, AppAttestationPurpose]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[AppAttestationResponse200, Error]
    """

    return sync_detailed(
        app_path=app_path,
        client=client,
        nonce=nonce,
        app_query=app_query,
        app_hash=app_hash,
        purpose=purpose,
    ).parsed


async def asyncio_detailed(
    app_path: str,
    *,
    client: AuthenticatedClient,
    nonce: str,
    app_query: Union[Unset, str] = UNSET,
    app_hash: Union[Unset, str] = UNSET,
    purpose: Union[Unset, AppAttestationPurpose] = UNSET,
) -> Response[Union[AppAttestationResponse200, Error]]:
    """Every replica's receipt-key attestation

     Each replica's `/attestation?purpose=receipts` for `nonce`. It reads the query `/attestation` does,
    but only `nonce`: `app`, `appHash` and `purpose` are ignored here.

    **Who:** whoever the app's access admits (a public app: anyone).

    Args:
        app_path (str):
        nonce (str):
        app_query (Union[Unset, str]):
        app_hash (Union[Unset, str]):
        purpose (Union[Unset, AppAttestationPurpose]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[AppAttestationResponse200, Error]]
    """

    kwargs = _get_kwargs(
        app_path=app_path,
        nonce=nonce,
        app_query=app_query,
        app_hash=app_hash,
        purpose=purpose,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    app_path: str,
    *,
    client: AuthenticatedClient,
    nonce: str,
    app_query: Union[Unset, str] = UNSET,
    app_hash: Union[Unset, str] = UNSET,
    purpose: Union[Unset, AppAttestationPurpose] = UNSET,
) -> Optional[Union[AppAttestationResponse200, Error]]:
    """Every replica's receipt-key attestation

     Each replica's `/attestation?purpose=receipts` for `nonce`. It reads the query `/attestation` does,
    but only `nonce`: `app`, `appHash` and `purpose` are ignored here.

    **Who:** whoever the app's access admits (a public app: anyone).

    Args:
        app_path (str):
        nonce (str):
        app_query (Union[Unset, str]):
        app_hash (Union[Unset, str]):
        purpose (Union[Unset, AppAttestationPurpose]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[AppAttestationResponse200, Error]
    """

    return (
        await asyncio_detailed(
            app_path=app_path,
            client=client,
            nonce=nonce,
            app_query=app_query,
            app_hash=app_hash,
            purpose=purpose,
        )
    ).parsed
