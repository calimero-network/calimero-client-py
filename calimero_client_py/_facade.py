"""What the generated facade (`calimero_client_py.facade`) runs on.

Each facade method names its `Operation` (the generated call and how the
transport treats it) and hands over the path's arguments, the query's and
the JSON body's fields as given; `Group._run` builds the generated request,
sends it through the client's sealed transport with the step-ups the client
answers, raises a refusal as `CloudError`, and answers the generated model
of the success (or, for the deprecated methods, the JSON as it came).
"""

import io
import json
from urllib.parse import quote
from enum import Enum
from typing import Any, Dict, Optional, Tuple

from .openapi.types import UNSET, File, Unset
from .sealed import error_from

__all__ = ["UNSET", "Group", "Operation"]


class Operation:
    """A generated call and how the transport treats it."""

    def __init__(
        self,
        module: Any,
        operation_id: str,
        *,
        body: Any = "none",
        answer: str = "json",
        confirm: bool = False,
        operator: bool = False,
        leader: bool = False,
        once: bool = False,
        repeatable: bool = False,
        retry_requires: Optional[str] = None,
    ):
        self.module = module
        self.operation_id = operation_id
        self.body = body
        self.answer = answer
        self.confirm = confirm
        self.operator = operator
        self.leader = leader
        self.once = once
        self.repeatable = repeatable
        # Safe to send again (and to the next node) only once this body
        # field is set: a call's idempotency key.
        self.retry_requires = retry_requires

    @staticmethod
    def enum(cls: Any, value: Any) -> Any:
        """A query value as the generated call takes it (an enum member)."""
        if isinstance(value, Unset) or value is None or isinstance(value, cls):
            return value
        return cls(value)


def _plain(value: Any) -> Any:
    """A value as JSON: a generated model by its `to_dict`, an enum by its value."""
    if hasattr(value, "to_dict"):
        return value.to_dict()
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, (list, tuple)):
        return [_plain(v) for v in value]
    if isinstance(value, dict):
        return {k: _plain(v) for k, v in value.items()}
    return value


class _Raw:
    """A JSON body as given, for a generated call (it reads `to_dict()`)."""

    def __init__(self, fields: Dict[str, Any]):
        self._fields = fields

    def to_dict(self) -> Dict[str, Any]:
        return self._fields


def _reads(model: Any, value: Any) -> bool:
    try:
        model.from_dict(dict(value))
        return True
    except (KeyError, ValueError, TypeError, AttributeError):
        return False


def _is_passkey(value: Any) -> bool:
    return callable(getattr(value, "get", None)) and callable(
        getattr(value, "create", None)
    )


class Group:
    """The operations of one tag, on a client. With `as_json`, a method answers
    the node's JSON as it came rather than the generated model (what the
    deprecated methods return)."""

    def __init__(self, client: Any, as_json: bool = False):
        self._client = client
        self._as_json = as_json

    def _run(
        self,
        op: Operation,
        path: Tuple[Any, ...],
        query: Dict[str, Any],
        body: Any,
        confirm: Any = None,
        operator_proof: Any = None,
    ) -> Any:
        client = self._client
        passkey = None
        options: Dict[str, Any] = {}
        fields: Optional[Dict[str, Any]] = None
        if isinstance(body, dict):
            fields = {k: _plain(v) for k, v in body.items() if not isinstance(v, Unset)}
        if op.confirm and confirm is not False:
            if confirm is None:
                passkey = client._passkey
            elif _is_passkey(confirm):
                passkey = confirm
            elif fields is not None:
                fields["confirm"] = _plain(confirm)
        if op.operator and operator_proof is not False:
            if operator_proof is None:
                options["operator_signed"] = True
            elif isinstance(operator_proof, str):
                options["operator_key"] = operator_proof
            elif fields is not None:
                fields["operatorProof"] = _plain(operator_proof)
        if op.leader:
            options["follow_leader"] = True
        if op.once:
            options["once"] = True
        if op.repeatable:
            options["repeatable"] = True
        if op.retry_requires and fields is not None and fields.get(op.retry_requires):
            options.update(repeatable=True, retry_unavailable=True)

        kwargs = {k: v for k, v in query.items() if not isinstance(v, Unset)}
        if op.body == "bytes":
            kwargs["body"] = File(payload=io.BytesIO(bytes(body)))
        elif op.body == "text":
            kwargs["body"] = body
        elif isinstance(op.body, tuple):
            # One of several shapes: the body as given, if one of them reads it.
            whole = _plain(body)
            if not self._as_json and not any(_reads(m, whole) for m in op.body):
                raise ValueError(
                    f"{op.operation_id}: the body is none of "
                    + ", ".join(m.__name__ for m in op.body)
                )
            kwargs["body"] = _Raw(whole)
        elif op.body != "none":
            # The generated model checks the body (a value the document does
            # not allow is refused here), but the body sent is the fields as
            # given: the model's `to_dict` sends an absent optional list as
            # `[]` (openapi-python-client 0.26), which a node reads otherwise
            # than an absent field (`scope_apps`' `apps`).
            try:
                op.body.from_dict(dict(fields or {}))
            except (KeyError, ValueError, TypeError, AttributeError):
                # The deprecated methods send what they always sent (a `null`
                # where the document names an enum, say), which the node
                # takes; a facade call is held to the document.
                if not self._as_json:
                    raise
            kwargs["body"] = _Raw(fields or {})

        api = client.api(passkey, **options)
        # The generated calls (openapi-python-client 0.26) put a path argument
        # in the URL as it is: each is one segment here, so an id can never
        # reach another route.
        segments = tuple(quote(str(p), safe="") for p in path)
        request = op.module._get_kwargs(*segments, **kwargs)
        response = api.get_httpx_client().request(**request)
        status = int(response.status_code)
        if status >= 400:
            raise error_from(status, response.content)
        if not self._as_json:
            return op.module._parse_response(client=api, response=response)
        if op.answer == "text":
            return response.content.decode("utf-8", errors="replace")
        if op.answer == "bytes":
            return bytes(response.content)
        if not response.content:
            return None
        try:
            return json.loads(response.content)
        except ValueError:
            return None
