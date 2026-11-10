"""The runtime a route layer sends through.

`Client` is two layers. The runtime is everything that is not a route: core's
Rust `calimero-client` (the node's attestation, the sealed channel, the
session's refresh), moving between nodes, the `412 CONFIRM_REQUIRED` passkey
step-up and error mapping. Its whole surface toward routes is one method,
`Client.request`, which `Transport` describes. The route methods
(`create_org`, `call`, `upload_blob`, ...) are the hand-written layer on top;
a route layer generated from the node's OpenAPI document needs nothing else:

    from calimero import Client
    from calimero.transport import Transport, route_path

    def get_app(t: Transport, app: str) -> dict:
        return t.request("GET", route_path("apps/{app}", app=app))

    get_app(Client("http://127.0.0.1:2501", allow_mock=True), app_id)
"""

from typing import Any, Mapping, Optional, Protocol, Sequence, Union
from urllib.parse import quote

__all__ = ["Transport", "route_path"]

QueryValue = Union[str, int, float, bool, None]


class Transport(Protocol):
    """What a route layer calls. `Client` (also `CloudClient`) implements it."""

    def request(
        self,
        method: str,
        path: str,
        *,
        query: Optional[Mapping[str, Union[QueryValue, Sequence[QueryValue]]]] = None,
        body: Any = None,
        passkey: Any = None,
        repeatable: Optional[bool] = None,
    ) -> Any:
        """Send `method path` (`GET`, `POST`, `PUT` or `DELETE`, the path
        relative to the node) through the attested, sealed channel with the
        client's credential, and answer the node's JSON. `query` is appended,
        encoded (`None` left out, a list repeated); `body` goes as JSON;
        `passkey` (a `SoftPasskey`) answers a `412 CONFIRM_REQUIRED`;
        `repeatable` lets a `503` or an unreachable node be retried (default:
        only a GET). A refusal raises `CloudError`."""
        ...


def route_path(template: str, **params: Union[str, int]) -> str:
    """`template` (`"apps/{app}/blobs/{id}"`) with each `{name}` replaced by
    `params[name]`, encoded as one path segment so an id cannot reach another
    route. A missing name raises `KeyError`."""
    out = []
    rest = template
    while True:
        start = rest.find("{")
        if start < 0:
            out.append(rest)
            return "".join(out)
        end = rest.index("}", start)
        out.append(rest[:start])
        out.append(quote(str(params[rest[start + 1 : end]]), safe=""))
        rest = rest[end + 1 :]
