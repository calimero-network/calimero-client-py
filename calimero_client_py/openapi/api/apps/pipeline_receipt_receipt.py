from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from ...models.error import Error
from ...models.pipeline_receipt_receipt_response_200 import (
    PipelineReceiptReceiptResponse200,
)
from typing import cast


def _get_kwargs(
    app: str,
    run: str,
) -> dict[str, Any]:

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/apps/{app}/pipeline-runs/{run}/receipt".format(
            app=app,
            run=run,
        ),
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Error, PipelineReceiptReceiptResponse200]]:
    if response.status_code == 200:
        response_200 = PipelineReceiptReceiptResponse200.from_dict(response.json())

        return response_200

    if response.status_code == 401:
        response_401 = Error.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = Error.from_dict(response.json())

        return response_403

    if response.status_code == 404:
        response_404 = Error.from_dict(response.json())

        return response_404

    if response.status_code == 409:
        response_409 = Error.from_dict(response.json())

        return response_409

    if response.status_code == 421:
        response_421 = Error.from_dict(response.json())

        return response_421

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Error, PipelineReceiptReceiptResponse200]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    app: str,
    run: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Error, PipelineReceiptReceiptResponse200]]:
    """A pipeline run's receipt

     The owner and the organization's members. For an ended run: the receipt, the answering node and its
    signature over it, by the node's receipt key.

    **Who:** signed in (a session or an API key).

    Args:
        app (str):
        run (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Error, PipelineReceiptReceiptResponse200]]
    """

    kwargs = _get_kwargs(
        app=app,
        run=run,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    app: str,
    run: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Error, PipelineReceiptReceiptResponse200]]:
    """A pipeline run's receipt

     The owner and the organization's members. For an ended run: the receipt, the answering node and its
    signature over it, by the node's receipt key.

    **Who:** signed in (a session or an API key).

    Args:
        app (str):
        run (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Error, PipelineReceiptReceiptResponse200]
    """

    return sync_detailed(
        app=app,
        run=run,
        client=client,
    ).parsed


async def asyncio_detailed(
    app: str,
    run: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Error, PipelineReceiptReceiptResponse200]]:
    """A pipeline run's receipt

     The owner and the organization's members. For an ended run: the receipt, the answering node and its
    signature over it, by the node's receipt key.

    **Who:** signed in (a session or an API key).

    Args:
        app (str):
        run (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Error, PipelineReceiptReceiptResponse200]]
    """

    kwargs = _get_kwargs(
        app=app,
        run=run,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    app: str,
    run: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Error, PipelineReceiptReceiptResponse200]]:
    """A pipeline run's receipt

     The owner and the organization's members. For an ended run: the receipt, the answering node and its
    signature over it, by the node's receipt key.

    **Who:** signed in (a session or an API key).

    Args:
        app (str):
        run (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Error, PipelineReceiptReceiptResponse200]
    """

    return (
        await asyncio_detailed(
            app=app,
            run=run,
            client=client,
        )
    ).parsed
