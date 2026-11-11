from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ...client import AuthenticatedClient, Client
from ...types import Response, UNSET
from ... import errors

from ...models.error import Error
from ...models.ops_quotas_set_body import OpsQuotasSetBody
from ...models.ops_quotas_set_response_200 import OpsQuotasSetResponse200
from typing import cast


def _get_kwargs(
    *,
    body: OpsQuotasSetBody,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/ops/quotas",
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Error, OpsQuotasSetResponse200]]:
    if response.status_code == 200:
        response_200 = OpsQuotasSetResponse200.from_dict(response.json())

        return response_200

    if response.status_code == 400:
        response_400 = Error.from_dict(response.json())

        return response_400

    if response.status_code == 401:
        response_401 = Error.from_dict(response.json())

        return response_401

    if response.status_code == 403:
        response_403 = Error.from_dict(response.json())

        return response_403

    if response.status_code == 409:
        response_409 = Error.from_dict(response.json())

        return response_409

    if response.status_code == 428:
        response_428 = Error.from_dict(response.json())

        return response_428

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Error, OpsQuotasSetResponse200]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    body: OpsQuotasSetBody,
) -> Response[Union[Error, OpsQuotasSetResponse200]]:
    """Change quotas

     Changes the limits the body names, the others keep their value; signed over `cluster / __set_quotas
    / { quotas }` with every limit after the change. With `org`: one payer's `maxConcurrentRuns`,
    `computeFuel` (decimal string) and `maxFileBytes` (`null` clears one). A name that is no quota is
    refused.

    **Who:** the ops token. **Operator signature:** answers `428 OPERATOR_SIGNATURE_REQUIRED` first;
    send it again with `operatorProof`, the operator key's signature over `details.message` and its
    `nonce`.

    Args:
        body (OpsQuotasSetBody): Other properties than those listed may be present: the ones
            listed are those clients rely on.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Error, OpsQuotasSetResponse200]]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    body: OpsQuotasSetBody,
) -> Optional[Union[Error, OpsQuotasSetResponse200]]:
    """Change quotas

     Changes the limits the body names, the others keep their value; signed over `cluster / __set_quotas
    / { quotas }` with every limit after the change. With `org`: one payer's `maxConcurrentRuns`,
    `computeFuel` (decimal string) and `maxFileBytes` (`null` clears one). A name that is no quota is
    refused.

    **Who:** the ops token. **Operator signature:** answers `428 OPERATOR_SIGNATURE_REQUIRED` first;
    send it again with `operatorProof`, the operator key's signature over `details.message` and its
    `nonce`.

    Args:
        body (OpsQuotasSetBody): Other properties than those listed may be present: the ones
            listed are those clients rely on.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Error, OpsQuotasSetResponse200]
    """

    return sync_detailed(
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    body: OpsQuotasSetBody,
) -> Response[Union[Error, OpsQuotasSetResponse200]]:
    """Change quotas

     Changes the limits the body names, the others keep their value; signed over `cluster / __set_quotas
    / { quotas }` with every limit after the change. With `org`: one payer's `maxConcurrentRuns`,
    `computeFuel` (decimal string) and `maxFileBytes` (`null` clears one). A name that is no quota is
    refused.

    **Who:** the ops token. **Operator signature:** answers `428 OPERATOR_SIGNATURE_REQUIRED` first;
    send it again with `operatorProof`, the operator key's signature over `details.message` and its
    `nonce`.

    Args:
        body (OpsQuotasSetBody): Other properties than those listed may be present: the ones
            listed are those clients rely on.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Error, OpsQuotasSetResponse200]]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    body: OpsQuotasSetBody,
) -> Optional[Union[Error, OpsQuotasSetResponse200]]:
    """Change quotas

     Changes the limits the body names, the others keep their value; signed over `cluster / __set_quotas
    / { quotas }` with every limit after the change. With `org`: one payer's `maxConcurrentRuns`,
    `computeFuel` (decimal string) and `maxFileBytes` (`null` clears one). A name that is no quota is
    refused.

    **Who:** the ops token. **Operator signature:** answers `428 OPERATOR_SIGNATURE_REQUIRED` first;
    send it again with `operatorProof`, the operator key's signature over `details.message` and its
    `nonce`.

    Args:
        body (OpsQuotasSetBody): Other properties than those listed may be present: the ones
            listed are those clients rely on.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Error, OpsQuotasSetResponse200]
    """

    return (
        await asyncio_detailed(
            client=client,
            body=body,
        )
    ).parsed
