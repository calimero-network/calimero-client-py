"""`Client` (also `CloudClient`): a client for one Calimero 1.0 cluster.

The routes are `calimero_client_py.openapi`, generated from core's OpenAPI
document; `Client.api()` gives the generated calls a client whose transport
seals every request to the node's attested key (`calimero_client_py.sealed`).
What is not a route is the native `SealedClient`'s: the attestation and the
sealed channel, the move between nodes and the refresh of a session, the event
streams, and the keys the client signs with.

The methods earlier releases had, one per route, are kept with their names
and signatures, each a thin call onto the generated client, and deprecated in
favour of calling it directly (`DeprecationWarning`); the passkey ceremonies,
the streams, the generic path calls and the key helpers are not routes and
are not deprecated.
"""

import base64
import functools
import json
import os
import warnings
from typing import Any, Callable, Dict, List, Optional, Tuple

from . import _native, facade
from ._facade import Group
from ._native import CloudError
from .openapi import AuthenticatedClient
from .sealed import error_from, sealed_client

__all__ = ["Client", "CloudClient"]

_SealedClient = _native.SealedClient

# The fields of a session, as earlier releases answered it (core's `Session`).
_SESSION = ("accountId", "sessionToken", "expiresAt", "refreshToken")


def deprecated(replacement: str) -> Callable:
    """Mark a route method as superseded by the facade method `replacement`
    (`"orgs.invite_create"`: `client.orgs.invite_create(...)`, one method per
    operation of core's OpenAPI document): a `DeprecationWarning` when called,
    and a note in its docstring. The signature is the method's own
    (`functools.wraps`)."""

    def mark(method: Callable) -> Callable:
        name = method.__name__
        calls = " or ".join(f"client.{r.strip()}(...)" for r in replacement.split("/"))

        @functools.wraps(method)
        def warned(*args: Any, **kwargs: Any) -> Any:
            warnings.warn(
                f"Client.{name} is deprecated: use {calls}",
                DeprecationWarning,
                stacklevel=2,
            )
            return method(*args, **kwargs)

        warned.__doc__ = (method.__doc__ or "").rstrip() + (
            f"\n\n        Deprecated: use ``{calls}`` (keyword arguments; it returns "
            f"the generated model of the answer, and a passkey approves it as "
            f"``confirm=passkey`` or from ``client.use_passkey``).\n"
        )
        warned.__deprecated__ = replacement
        return warned

    return mark


def _confirm(passkey: Any) -> Any:
    """The facade's `confirm=` for a method's `passkey`: none given is none
    used (not the client's own, which these methods never used)."""
    return False if passkey is None else passkey


class _GroupAttribute:
    """`client.<group>`: the facade's operations of one tag on this client.
    `legacy`, when the name was a method already (`packages()`), is what
    calling the group does, and what the attribute is on the class."""

    def __init__(self, name: str, legacy: Optional[Callable] = None):
        self.name = name
        self.legacy = legacy
        self.__doc__ = facade.GROUPS[name].__doc__

    def __get__(self, client: Any, owner: Any = None) -> Any:
        if client is None:
            return self.legacy if self.legacy is not None else self
        group = facade.GROUPS[self.name](client)
        if self.legacy is not None:
            return _CallableGroup(group, functools.partial(self.legacy, client))
        return group


class _JsonGroups:
    """`client.json`: each facade group, answering JSON."""

    def __init__(self, client: Any):
        self._client = client

    def __getattr__(self, name: str) -> Any:
        group = facade.GROUPS.get(name)
        if group is None:
            raise AttributeError(name)
        return group(self._client, as_json=True)


class _CallableGroup:
    """A group whose name was a method: attributes are the group's, a call is
    the method's."""

    def __init__(self, group: Any, call: Callable):
        self._group = group
        self._call = call

    def __getattr__(self, name: str) -> Any:
        return getattr(self._group, name)

    def __call__(self, *args: Any, **kwargs: Any) -> Any:
        return self._call(*args, **kwargs)


def _answer(response: Any) -> Any:
    """A generated call's answer as JSON, or the error it stands for."""
    status = int(response.status_code)
    if status >= 400:
        raise error_from(status, response.content)
    if not response.content:
        return None
    try:
        return json.loads(response.content)
    except ValueError:
        return None


def _field(v: Any, name: str) -> str:
    if isinstance(v, dict) and isinstance(v.get(name), str):
        return v[name]
    raise ValueError(f"the node's answer has no {name}")


def _str(v: Any, name: str) -> str:
    """`v[name]` as text, `""` when absent (as core's client reads ids)."""
    found = v.get(name) if isinstance(v, dict) else None
    return found if isinstance(found, str) else ""


def _session(v: Any) -> Dict[str, Any]:
    if not isinstance(v, dict) or any(k not in v for k in _SESSION):
        raise CloudError("the node's answer is not a session", 0, "TRANSPORT", "null")
    return {k: v[k] for k in _SESSION}


def _json_arg(v: Any) -> Any:
    """An argument the client sends as JSON: `None` is `{}`, as it always was."""
    return {} if v is None else v


class Client:
    """A client for one node of a Calimero 1.0 cluster (and, given `nodes`,
    the others it may move to).

    Connect to `base_url`. The node is trusted only once its attestation
    verifies; `allow_mock` accepts development (mock) quotes, `app_hash`
    (hex) pins the bundle the quote must bind.

    `nodes` are other nodes of the same cluster, in the order to move to
    them when a node answers `421 WRONG_NODE` (its release may not serve
    the account); each is attested under the same policy before anything
    is sent to it. Without them that answer is raised as `WrongNode`.
    A node that cannot be reached, or that keeps answering `503` to a read
    or a call, is left for the next too (each node once per request).
    A `401` to a request made with a session is answered by one refresh
    (any node takes it) and the request sent again; `LoginRequired` is
    raised only when that does not help.

    A real TDX quote is accepted only from an image the policy names:
    `tee_profiles` admits nodes whose signed mero-tee release lists their
    measurements under one of those profiles (`min_release` is a floor).
    Without it, only mock quotes (`allow_mock`) can be accepted. A
    release's debug profiles (whose memory the host can read) are refused
    unless `allow_debug_profiles`.

    The routes: `client.api()` is a client for the generated calls in
    `calimero_client_py.openapi.api`, sealed to the node:

        from calimero_client_py.openapi.api.organizations import org_create
        from calimero_client_py.openapi.models import OrgCreateBody
        org = org_create.sync(client=client.api(), body=OrgCreateBody(name="Acme"))
    """

    def __init__(
        self,
        base_url: str,
        allow_mock: bool = False,
        app_hash: Optional[str] = None,
        tee_profiles: Optional[List[str]] = None,
        min_release: Optional[str] = None,
        allow_debug_profiles: bool = False,
        nodes: Optional[List[str]] = None,
    ):
        self._init(
            _SealedClient(
                base_url,
                allow_mock,
                app_hash,
                tee_profiles,
                min_release,
                allow_debug_profiles,
                nodes,
            )
        )

    def _init(self, native: Any) -> None:
        self._native = native
        self._passkey: Any = None
        self._operator_signer: Any = None
        self._clients: Dict[Tuple[Any, ...], AuthenticatedClient] = {}

    @classmethod
    def connect(
        cls,
        cluster: str,
        *,
        tee_profiles: Optional[List[str]] = None,
        min_release: Optional[str] = None,
        allow_mock: bool = False,
        app_hash: Optional[str] = None,
        allow_debug_profiles: bool = False,
        directory: Optional[str] = None,
        zone: Optional[str] = None,
    ) -> "Client":
        """Connect to the cluster named `cluster`, by the nodes the Calimero
        Cloud node directory lists for it (`GET {directory}/{cluster}`).

        The nodes in `zone` come first, then the rest, each group in a
        weighted random order; the first is the node in use and the others
        are `nodes`. The directory is not trusted: every node is attested
        under this policy (the arguments mean what they do for the
        constructor) before anything is sent to it. The listing is fetched
        again once it is due and after the client moved off an unavailable
        node. Raises `CloudError` (code `DIRECTORY`; the status is the
        directory's, `404` for an unknown cluster, 0 when it could not be
        reached) when the directory lists no usable node.
        """
        client = cls.__new__(cls)
        client._init(
            _SealedClient.connect(
                cluster,
                tee_profiles=tee_profiles,
                min_release=min_release,
                allow_mock=allow_mock,
                app_hash=app_hash,
                allow_debug_profiles=allow_debug_profiles,
                directory=directory,
                zone=zone,
            )
        )
        return client

    # ------------------------------------------------------------ the generated client

    def api(
        self,
        passkey: Any = None,
        *,
        operator_signed: bool = False,
        operator_key: Optional[str] = None,
        operator_signer: Optional[Callable[[Dict[str, Any]], Any]] = None,
        follow_leader: bool = False,
        repeatable: bool = False,
        retry_unavailable: bool = False,
        once: bool = False,
    ) -> AuthenticatedClient:
        """A client for the generated calls (`calimero_client_py.openapi.api`),
        whose requests go through the sealed channel with this client's
        credential (`use_api_key`, a session).

        `passkey` answers a `412 CONFIRM_REQUIRED` (what an owner approves):
        the request is sent again with its assertion over the node's
        challenge. `operator_signed` answers a `428
        OPERATOR_SIGNATURE_REQUIRED` with the signer `use_operator_signer` set,
        else the key `use_operator_key` set (or `operator_signer`, or
        `operator_key`, a seed in hex). `follow_leader` sends a request a node
        answers `421 NOT_LEADER` to the leader it names (the Ops API's replica
        moves and node removal). `repeatable` marks requests safe to send
        twice (a GET always is), `retry_unavailable` sends them again to the
        same node while it answers `503` (a call with an idempotency key),
        `once` sends them once (a sign-in or sign-up).
        """
        if operator_signed and operator_key is None and operator_signer is None:
            operator_signer = self._operator_signer
        options = dict(
            operator_signed=operator_signed,
            operator_key=operator_key,
            operator_signer=operator_signer,
            follow_leader=follow_leader,
            repeatable=repeatable,
            retry_unavailable=retry_unavailable,
            once=once,
        )
        if passkey is not None:
            return sealed_client(self._native, passkey, **options)
        key = tuple(sorted(options.items(), key=lambda kv: kv[0]))
        client = self._clients.get(key)
        if client is None:
            client = self._clients[key] = sealed_client(self._native, **options)
        return client

    def _op(
        self,
        operation_id: str,
        *path: Any,
        query: Optional[Dict[str, Any]] = None,
        body: Any = None,
        confirm: Any = False,
        operator_proof: Any = None,
    ) -> Any:
        """Run the facade's operation `operation_id` with a JSON body as given,
        answering the node's JSON as it came (what the deprecated methods
        return). A passkey answers a step-up only when given (`confirm`), never
        the client's own (`use_passkey`), which these methods never used."""
        op = getattr(facade, "_" + operation_id.upper())
        return Group(self, as_json=True)._run(
            op, path, query or {}, body, confirm, operator_proof
        )

    def use_operator_signer(
        self, signer: Optional[Callable[[Dict[str, Any]], Any]]
    ) -> None:
        """Answer the node's `428 OPERATOR_SIGNATURE_REQUIRED` with
        `signer(details)`, the `operatorProof` (`{nonce, signature}`) for the
        statement `details` names (`{subject, method, args, nonce, cluster,
        message}`): for a key the client never holds, or a signer that checks
        the statement before it signs. It takes precedence over
        `use_operator_key`. `None` forgets it."""
        self._operator_signer = signer

    @property
    def json(self) -> Any:
        """The facade's groups answering the node's JSON as it came (dicts and
        lists) rather than the generated models: `client.json.ops.status()`.
        For a node older than the document, whose answers may lack a field the
        document requires."""
        return _JsonGroups(self)

    def use_passkey(self, passkey: Any) -> None:
        """Answer the node's `412 CONFIRM_REQUIRED` in the facade's methods
        (`client.orgs.invite_create(...)`) with `passkey` (anything with
        `create(options)` and `get(challenge)`, as `SoftPasskey`), unless a
        call passes its own `confirm=`. `None` forgets it."""
        self._passkey = passkey

    def _path(
        self,
        method: str,
        path: str,
        body: Any = None,
        *,
        json_body: bool = True,
        **options: Any,
    ) -> Any:
        """`method` on any API `path` through the sealed transport."""
        client = self.api(**options).get_httpx_client()
        kwargs: Dict[str, Any] = {}
        if json_body:
            kwargs["json"] = body
        response = client.request(method, "/" + path.lstrip("/"), **kwargs)
        return _answer(response)

    def _text(self, response: Any) -> str:
        """A text answer (`ops/metrics`); an error's text is its message."""
        status = int(response.status_code)
        text = response.content.decode("utf-8", errors="replace")
        if status >= 400:
            raise CloudError(f"{status} ERROR: {text}", status, "ERROR", "null")
        return text

    # ------------------------------------------------------------ the connection

    @property
    def node_url(self) -> str:
        """The URL of the node in use: `base_url`, until a `421 WRONG_NODE` or
        an unavailable node moved the client to the next of `nodes`."""
        return self._native.node_url

    @property
    def node_urls(self) -> List[str]:
        """Every node this client may use, in the order it moves through them
        (the node in use among them)."""
        return self._native.node_urls

    def attestation(self) -> Any:
        """The node's attestation document (unverified, for display)."""
        return self._native.attestation()

    def verify_app_cluster(self, app: str) -> Any:
        """Verify every replica of `app`: each one's quote over a fresh nonce,
        held to this client's policy, binding its receipt key and the
        cluster's governance. Returns `{"keys": {node_id: receipt_key_hex},
        "governance": {"releases", "appUpgrades", "outbound"}}`; a call's or
        view's `receipt` is to be trusted only from one of those keys."""
        return self._native.verify_app_cluster(app)

    # ------------------------------------------------------------ credentials

    def use_api_key(self, key: str) -> None:
        """Authenticate every request with an API key (`cal_...`)."""
        self._native.use_api_key(key)

    def use_session(self, session: Any) -> None:
        """Authenticate with a session returned by `sign_up`/`sign_in`."""
        self._native.use_session(session)

    def session(self) -> Any:
        """The session in use (`sign_up`, `sign_in`, `refresh`, `use_session`),
        or `None` (none yet, or an API key instead)."""
        return self._native.session()

    def refresh(self) -> Any:
        """Exchange the refresh token for a new session, on the node in use (any
        node of the cluster takes it). One at a time with the refreshes the
        client makes on its own after a `401`: a refresh token is used once."""
        return self._native.refresh()

    def use_operator_key(self, seed: Optional[str]) -> None:
        """Sign the Ops API's decisions with the operator's Ed25519 key (its
        32-byte seed, hex; the nodes were started with `--operator-key` set to
        its verify key): a `428 OPERATOR_SIGNATURE_REQUIRED` from `ops_*` is
        then answered with the signature. `None` forgets it."""
        self._native.use_operator_key(seed)

    @staticmethod
    def operator_public_key(seed: str) -> str:
        """The operator key's verify half (hex), what the nodes take as
        `--operator-key`, from its 32-byte seed (hex; `secrets.token_hex(32)`
        makes one)."""
        return _SealedClient.operator_public_key(seed)

    @staticmethod
    def api_key_public(key: str) -> str:
        """The Ed25519 public key (hex) an API key stands for, byte-for-byte
        core's `proof::api_key_public`: the seed is `SHA-256("calimero.api-key.v1\\0"
        || key)`. An upgrade key's record (`list_api_keys`) keeps it as
        `upgrades.proofKey`, and the node signs each upgrade the key makes
        with its private half, so the directory's log proves the key holder
        asked for it. Compare the two to check which key a record is."""
        return _SealedClient.api_key_public(key)

    @staticmethod
    def normalize_recovery_code(code: str) -> str:
        """The canonical form of a recovery code as written down: dashes and
        spaces dropped, upper case, `O` read as `0` and `I`/`L` as `1`.
        `ValueError` for anything that cannot be a code."""
        return _SealedClient.normalize_recovery_code(code)

    @staticmethod
    def recovery_code_key(code: str) -> str:
        """The Ed25519 public key (hex) a recovery code stands for: the seed is
        `SHA-256("calimero.account-recovery-code.v1\\0" || canonical code)`,
        core's `proof::recovery_key`. The node records only this; it signs a
        recovery's start with the private half. `ValueError` for anything that
        cannot be a code."""
        return _SealedClient.recovery_code_key(code)

    @staticmethod
    def new_recovery_seed() -> str:
        """A new recovery seed, as the owner writes it down (64 hex digits in
        groups of eight). Keep it offline; give the organization only
        `recovery_public_key(seed)`."""
        return _SealedClient.new_recovery_seed()

    @staticmethod
    def recovery_public_key(seed: str) -> str:
        """The public key (base64url) of a recovery seed: what the organization
        stores (`create_org(.., recovery_key=)`, `set_recovery_key`)."""
        return _SealedClient.recovery_public_key(seed)

    @staticmethod
    def sign_recovery(seed: str, org: str, account: str, nonce: str) -> str:
        """The seed's signature (base64url) handing `org` to `account` under
        `nonce`, as `start_recovery` sends it."""
        return _SealedClient.sign_recovery(seed, org, account, nonce)

    # ------------------------------------------------------------ passkey ceremonies

    def _signed_in(self, answer: Any) -> Dict[str, Any]:
        s = _session(answer)
        self._native.use_session(s)
        return s

    def sign_up(self, passkey: Any, name: str, origin: Optional[str] = None) -> Any:
        """Create an account with a new passkey and sign in. Returns the session.
        The node then asks the new passkey to sign the account it creates
        (`412 CONFIRM_REQUIRED`), which `passkey` answers; a node from before
        that answers the registration at once. `origin`: the web origin the
        passkey is made for (one a package registered, `add_package_origin`);
        without it, the node's own."""
        begin_body: Dict[str, Any] = {"name": name}
        if origin is not None:
            begin_body["origin"] = origin
        begin = self._op("register_begin", body=begin_body)
        registration = passkey.create(begin["options"])
        done = self._op(
            "register_finish",
            body={"challengeId": begin["challengeId"], "registration": registration},
            confirm=_confirm(passkey),
        )
        return self._signed_in(done)

    def sign_in(self, passkey: Any, origin: Optional[str] = None) -> Any:
        """Sign in with an existing passkey. Returns the session. `origin` as
        `sign_up`: the session is then the app's that registered it."""
        begin_body = {} if origin is None else {"origin": origin}
        begin = self._op("login_begin", body=begin_body)
        challenge = begin["options"]["challenge"]
        assertion = passkey.get(
            base64.urlsafe_b64decode(challenge + "=" * (-len(challenge) % 4))
        )
        done = self._op(
            "login_finish",
            body={"challengeId": begin["challengeId"], "assertion": assertion},
        )
        return self._signed_in(done)

    def add_passkey(self, passkey: Any, approver: Any) -> str:
        """Add `passkey` (another device) to the signed-in account, approved
        with `approver`, a passkey the account already has (the node's `412
        CONFIRM_REQUIRED`). Returns the new credential's id."""
        if passkey is approver:
            raise ValueError(
                "approve a new passkey with one the account already has, not itself"
            )
        begin = self._op("passkey_add_begin")
        registration = passkey.create(begin["options"])
        done = self._op(
            "passkey_add_finish",
            body={"challengeId": begin["challengeId"], "registration": registration},
            confirm=_confirm(approver),
        )
        return _str(done, "credentialId")

    def start_account_recovery(self, code: str, passkey: Any) -> Any:
        """Start recovering the account `code` names, signed out, with `passkey`,
        a new one made here for the node's options (`/auth/recovery/begin`,
        then `/start`). Nothing is added and no session opens yet: answers the
        pending recovery, `{accountId, credentialId, startedAt, readyAt}`;
        `finish_account_recovery` with the same passkey once `readyAt` has
        passed (72 hours). The code is spent. A wrong or spent code raises
        `LoginRequired` (`401`), an account with a recovery under way
        `CloudError` `409 RECOVERY_PENDING`; a code that cannot be one,
        `ValueError` before anything is sent."""
        code = _SealedClient.normalize_recovery_code(code)
        begin = self._op(
            "account_recovery_begin",
            body={"code": code},
        )
        registration = passkey.create(begin["options"])
        return self._op(
            "account_recovery_start",
            body={
                "challengeId": begin["challengeId"],
                "code": code,
                "registration": registration,
            },
        )

    def finish_account_recovery(
        self, passkey: Any, account_id: Optional[str] = None
    ) -> Any:
        """Finish a recovery once its delay passed, signed out, with `passkey`,
        the one `start_account_recovery` registered: the node adds it, ends
        every session the account had, and signs in. Returns the session and
        uses it. `account_id` (the start's `accountId`) lets the node refuse
        early; without it the passkey names the recovery. Before `readyAt`:
        `CloudError` `409 RECOVERY_NOT_READY` (`details.readyAt`)."""
        body = {} if account_id is None else {"accountId": account_id}
        done = self._op(
            "account_recovery_finish",
            body=body,
            confirm=_confirm(passkey),
        )
        return self._signed_in(done)

    def start_recovery(self, org: str, seed: str) -> Any:
        """Start handing `org` to the signed-in account with the recovery
        `seed`, which signs here and is never sent. Answers `{ startedAt,
        readyAt }`; `finish_recovery` once `readyAt` has passed. A wrong seed
        raises `CloudError` (403 `WRONG_RECOVERY_KEY`)."""
        _SealedClient.recovery_public_key(seed)  # a seed, before anything is sent
        account = _field(self._op("me"), "accountId")
        nonce = base64.urlsafe_b64encode(os.urandom(16)).rstrip(b"=").decode()
        signature = _SealedClient.sign_recovery(seed, org, account, nonce)
        return self._op(
            "recovery_start",
            org,
            body={"nonce": nonce, "signature": signature},
        )

    # ------------------------------------------------------------ streams

    def events(
        self, app: str, start: int = 0, limit: int = 100, timeout: float = 5.0
    ) -> Any:
        """Committed events from index `start`: up to `limit` of them, or as many
        as arrive within `timeout` seconds. Each is `{"id", "kind", "data"}`."""
        return self._native.events(app, start, limit, timeout)

    def presence_stream(self, app: str, limit: int = 100, timeout: float = 5.0) -> Any:
        """Presence as a stream: the snapshot first (`{"kind": "snapshot",
        "data": [...]}`), then one `{"kind": "presence", "data": {...}}` per
        change; up to `limit` of them, or as many as arrive within `timeout`
        seconds."""
        return self._native.presence_stream(app, limit, timeout)

    # ------------------------------------------------------------ any path

    def get(self, path: str) -> Any:
        """GET any API path."""
        return self._path("GET", path, json_body=False)

    def get_text(self, path: str) -> str:
        """GET any API path whose answer is text (`ops/metrics`)."""
        client = self.api().get_httpx_client()
        return self._text(client.request("GET", "/" + path.lstrip("/")))

    def post(self, path: str, body: Any = None) -> Any:
        """POST any API path (organizations, teams, invites, ...)."""
        return self._path("POST", path, _json_arg(body))

    def put(self, path: str, body: Any = None) -> Any:
        """PUT any API path (an alias: `auth/aliases/{alias}` with `{"app": ...}`)."""
        return self._path("PUT", path, _json_arg(body))

    def delete(self, path: str, body: Any = None) -> Any:
        """DELETE any API path; `body`, when given, goes as JSON (a proof the
        caller made itself: `{"confirm": ...}` for a passkey, a member or a
        team row, `{"operatorProof": ...}` for `ops/nodes/{id}`). Without one
        the DELETE carries no body, as before."""
        if body is None:
            return self._path("DELETE", path, json_body=False)
        return self._path("DELETE", path, body)

    def post_confirmed(self, path: str, body: Any, passkey: Any) -> Any:
        """POST `body` to `path`, answering a `412 CONFIRM_REQUIRED` with
        `passkey`'s assertion over the node's challenge: what an owner
        approves (upgrades, organization upgrades, release and outbound
        approvals, deleting an organization's...)."""
        return self._path_confirmed("POST", path, _json_arg(body), passkey)

    def delete_confirmed(self, path: str, passkey: Any, body: Any = None) -> Any:
        """DELETE `path`, answering `412 CONFIRM_REQUIRED` (deleting an
        organization) with `passkey`, as `post_confirmed` does. `body`, when
        given, goes as JSON, and the approval is added to it (`{"origin": ...}`
        on `package-origins/{package}`)."""
        return self._path_confirmed("DELETE", path, _json_arg(body), passkey)

    def _path_confirmed(self, method: str, path: str, body: Any, passkey: Any) -> Any:
        client = self.api(passkey).get_httpx_client()
        return _answer(client.request(method, "/" + path.lstrip("/"), json=body))

    def post_operator_signed(
        self, path: str, body: Any = None, operator_key: Optional[str] = None
    ) -> Any:
        """POST an operator decision to `path`, answering `428
        OPERATOR_SIGNATURE_REQUIRED` with `operator_key` (a seed, hex), else
        the key `use_operator_key` set."""
        return self._path(
            "POST",
            path,
            _json_arg(body),
            operator_signed=True,
            operator_key=operator_key,
        )

    def delete_operator_signed(
        self, path: str, operator_key: Optional[str] = None
    ) -> Any:
        """DELETE an operator decision (`ops/nodes/{id}`), signed as
        `post_operator_signed`."""
        return self._path(
            "DELETE", path, {}, operator_signed=True, operator_key=operator_key
        )

    # ------------------------------------------------------------ account (routes)

    @deprecated("auth.me")
    def me(self) -> Any:
        """Who the caller is (`GET /auth/me`): `accountId`, `name`, `passkeys`
        (each `{id, label, signCount, addedAt, lastSignInAt}`), `orgs` (each
        `{orgId, name, role}`), `via` (`"session"` or `"apiKey"`), `passkeyId`
        (the passkey this session was opened with; `None` for an API key), and
        `app` and `origin` for a session signed in from a package's origin.

        A passkey's `addedAt` and `lastSignInAt` (Unix seconds; core #4894)
        say when it was added and last signed in (a sign-up, a sign-in or an
        account recovery's finish; a refresh does not move it). `None` for
        what happened on a release that kept no times; absent on such a node
        (1.0.0-rc.14 and earlier)."""
        return self._op("me")

    @deprecated("auth.apikey_create")
    def create_api_key(self, options: Any, passkey: Any) -> Tuple[str, str]:
        """Create an API key, approved with the account's `passkey` (the node's
        `412 CONFIRM_REQUIRED`; a session's, never an API key's). Returns
        `(key_id, key)`; the key is shown once. `options` (or `None`) may set
        `label`, `apps` and `methods` (scopes), `readOnly`, `allowConfirm` and
        `ttlSecs` (at most a year).

        `upgrades: {"app", "package"?}` makes an upgrade key, for CI (core
        1.0.0-rc.10): it upgrades that one app to signed releases of its
        package (default: the one the app runs, by that package's signer
        pinned now) with `upgrade_app(app, version=...)` and no passkey, and
        every other request with it is refused `403 UPGRADE_KEY_ONLY`. It
        takes no other scope field (`400 BAD_UPGRADE_KEY`); the caller must
        be one who may upgrade the app. Its record in `list_api_keys` names
        the key's public half, `upgrades.proofKey` (`api_key_public(key)`)."""
        v = self._op(
            "apikey_create",
            body=_json_arg(options),
            confirm=_confirm(passkey),
        )
        return _str(v, "keyId"), _str(v, "key")

    @deprecated("auth.logout")
    def sign_out(self) -> None:
        """End this session on the node (its refresh token stops working)."""
        self._op("logout")

    @deprecated("auth.logout_all")
    def sign_out_everywhere(self) -> None:
        """End every session of this account, on every device."""
        self._op("logout_all")

    @deprecated("auth.sessions_list")
    def sessions(self) -> Any:
        """The account's live sessions (`GET /auth/sessions`, core #4894), latest
        active first, at most 256: a list of `{session, passkey, app, origin,
        openedAt, lastActiveAt, current}`. `passkey` is the passkey it was
        opened with, `app` and `origin` the package and registered origin it
        is bound to (`None`: the cluster's own origin), `openedAt` when it
        opened (`None`: before the node kept it), `current` marks this one.
        `lastActiveAt` is when its refresh token was last exchanged; the
        session may have been used up to about 1 h 15 min later. The account
        holder's own session on the cluster's origin only: an API key or a
        package-bound session is refused `403`. A node without the route
        answers a bare `404`."""
        return (self._op("sessions_list") or {}).get("sessions")

    @deprecated("auth.session_revoke")
    def revoke_session(self, session: str) -> None:
        """End one of the account's sessions (`DELETE /auth/sessions/{session}`),
        a lost device's say, from another; no passkey is asked. A session the
        account does not hold live (ended already, or another account's) is
        `404 NOT_FOUND`; gated as `sessions`. Ending this client's own
        session leaves it holding dead tokens: use `sign_out` for that."""
        self._op("session_revoke", session)

    @deprecated("auth.activity_list")
    def activity(self, after: Optional[int] = None) -> Any:
        """What the account's passkeys did (`GET /auth/activity`, core #4901),
        newest first, the last 100: a list of `{at, passkey, kind, app,
        origin, method, subject}`. `kind` is `"signIn"` (a sign-up, a sign-in
        or a recovery's finish; a refresh is none), with the package `app` and
        registered `origin` its session is bound to (`None`: the cluster's own
        origin), or `"approval"` (a change the passkey approved with a
        step-up), with the `method` and `subject` its consent states. `after`
        (Unix seconds) keeps only what came later. Gated as `sessions`: the
        account holder's own session on the cluster's origin only. A node
        without the route answers a bare `404`."""
        if after is None:
            v = self._op("activity_list")
        else:
            v = self._op("activity_list", query={"after": after})
        return (v or {}).get("activity")

    @deprecated("auth.passkey_remove")
    def remove_passkey(self, credential_id: str, passkey: Any) -> None:
        """Remove a passkey (a device) from the signed-in account, approved with
        `passkey`, one the account has."""
        self._op(
            "passkey_remove",
            credential_id,
            body={},
            confirm=_confirm(passkey),
        )

    @deprecated("auth.apikey_list")
    def list_api_keys(self) -> Any:
        """The account's API keys (their records; the keys themselves are never shown again)."""
        return (self._op("apikey_list") or {}).get("keys")

    @deprecated("auth.apikey_revoke")
    def revoke_api_key(self, key_id: str, passkey: Any) -> None:
        """Revoke an API key by its id, approved with the account's `passkey`
        (the node's `412 CONFIRM_REQUIRED`)."""
        self._op(
            "apikey_revoke",
            key_id,
            body={},
            confirm=_confirm(passkey),
        )

    @deprecated("auth.usage")
    def usage(self) -> Any:
        """What this account uses, with the cluster's quotas."""
        return self._op("usage")

    @deprecated("auth.passkey_label")
    def label_passkey(self, credential_id: str, label: str) -> None:
        """Name one of this account's passkeys; `""` clears the name."""
        self._op("passkey_label", credential_id, body={"label": label})

    @deprecated("auth.alias_set")
    def set_alias(self, alias: str, app: str) -> None:
        """Name `app` for this account (`alias` then reaches it wherever an app id does)."""
        self._op("alias_set", alias, body={"app": app})

    @deprecated("auth.alias_get")
    def alias(self, alias: str) -> Any:
        """The app an alias names: `{"alias", "appId"}`."""
        return self._op("alias_get", alias)

    @deprecated("auth.alias_list")
    def aliases(self) -> Any:
        """Every alias of this account: `{alias: app_id}`."""
        return (self._op("alias_list") or {}).get("aliases")

    @deprecated("auth.alias_delete")
    def delete_alias(self, alias: str) -> None:
        """Remove an alias of this account."""
        self._op("alias_delete", alias)

    # ------------------------------------------------------------ account recovery (routes)

    @deprecated("auth.recovery_create")
    def create_recovery_code(self, passkey: Any) -> str:
        """Make (or replace) the signed-in account's recovery code, approved with
        `passkey`, one the account has (the node's `412 CONFIRM_REQUIRED`).
        Returns the code (`XXXX-XXXX-...`), shown once: write it down. A
        session's on the cluster's own origin, never an API key's."""
        v = self._op("recovery_create", body={}, confirm=_confirm(passkey))
        return _field(v, "code")

    @deprecated("auth.recovery_remove")
    def remove_recovery_code(self, passkey: Any) -> None:
        """Remove the account's recovery code, approved with `passkey`."""
        self._op("recovery_remove", body={}, confirm=_confirm(passkey))

    @deprecated("auth.account_recovery_cancel")
    def cancel_account_recovery(self, passkey: Any) -> None:
        """Cancel the recovery under way for the signed-in account, approved
        with `passkey`, one the account already has. The code stays spent."""
        self._op(
            "account_recovery_cancel",
            body={},
            confirm=_confirm(passkey),
        )

    # ------------------------------------------------------------ apps (routes)

    @deprecated("packages.bundle_put")
    def upload_bundle(self, wasm: bytes) -> str:
        """Upload a WASM bundle. Returns its hash."""
        return _str(self._op("bundle_put", body=wasm), "hash")

    @deprecated("apps.transfer")
    def transfer_app(self, app: str, to: str, passkey: Any) -> Any:
        """Hand `app` to the account `to` (an org app only to one of its members),
        approved with `passkey`. The app's owner may; for an organization's
        app, its owner only while a member of the organization (`member` or
        above), and the organization's admins (`admin`, `owner`) too, so an
        organization can hand on an app whose owner left (core #4893)."""
        return self._op("app_transfer", app, body={"to": to}, confirm=_confirm(passkey))

    @deprecated("apps.move")
    def move_app(
        self, app: str, to_org: str, passkey: Any, bring_members: bool = False
    ) -> Any:
        """Move `app` to the organization `to_org`, its data where it is
        (core `POST /apps/{app}/move`), approved with `passkey`. Whoever
        manages the app now (its owner while a member of its organization, an
        admin there, or a personal app's owner) moves it, as an admin or the
        owner of `to_org`. Its files, runs and quotas go to the target; `org`
        or app-team access becomes the target's app team of its package.
        `bring_members` adds the people of its organization who reach it to
        the target (capped at `member`). Answers `{appId, from, org, access,
        created, added, revokedInvites}`; a node from before the route answers
        a bare `404`."""
        body = {"toOrg": to_org, "bringMembers": bring_members}
        return self._op("app_move", app, body=body, confirm=_confirm(passkey))

    @deprecated("apps.presence_publish")
    def publish_presence(
        self, app: str, seq: int, state: Optional[bytes] = None
    ) -> Any:
        """Publish this caller's presence in `app`: `state` (bytes, at most 4 KiB),
        or `None` to retract it. The newest `seq` wins."""
        body = {"seq": seq, "state": None if state is None else bytes(state).hex()}
        return self._op("presence_publish", app, body=body)

    @deprecated("apps.presence_get")
    def presence(self, app: str) -> Any:
        """Who is present in `app` now."""
        return self._op("presence_get", app)

    @deprecated("apps.create")
    def create_app_from_package(
        self,
        name: str,
        package: str,
        version: Optional[str] = None,
        service: Optional[str] = None,
        options: Any = None,
    ) -> str:
        """Create an app from a package release (the newest without `version`),
        installed from the registry first if the cluster lacks it. `options`
        as `create_app`. Returns the app id."""
        body: Dict[str, Any] = {"name": name, "package": package}
        if version is not None:
            body["version"] = version
        if service is not None:
            body["service"] = service
        body.update(_json_arg(options))
        return _str(self._op("app_create", body=body), "appId")

    @deprecated("apps.create")
    def create_app(self, name: str, bundle: str, options: Any = None) -> str:
        """Create an app from a bundle. `options` may set `org`, `access`, `confirmMethods`."""
        body: Dict[str, Any] = {"name": name, "bundle": bundle}
        o = _json_arg(options)
        if isinstance(o, dict):
            body.update(o)
        return _str(self._op("app_create", body=body), "appId")

    @deprecated("apps.call")
    def call(
        self,
        app: str,
        method: str,
        args: Any = None,
        idempotency_key: Optional[str] = None,
    ) -> Any:
        """Call a method. Returns `{"index", "result", "events"}`. While the node
        answers 503 or cannot be reached, the call is sent again with the same
        idempotency key (`idempotency_key`, else one the client makes), so it
        runs once; when the node keeps failing, the call goes with that key to
        the client's next node (the key is the app's, cluster-wide)."""
        if idempotency_key is None:
            # A `node:` key is kept 10 minutes, long enough for the client's
            # own retries and failover (core's `NODE_KEY_PREFIX`).
            idempotency_key = (
                "node:c-"
                + base64.urlsafe_b64encode(os.urandom(16)).rstrip(b"=").decode()
            )
        body = {
            "method": method,
            "args": _json_arg(args),
            "idempotencyKey": idempotency_key,
        }
        v = self._op(
            "intents",
            app,
            body=body,
        )
        return _call_result(v)

    @deprecated("apps.call")
    def call_confirmed(self, app: str, method: str, args: Any, passkey: Any) -> Any:
        """Call a `confirm` method; `passkey` approves this exact call."""
        body = {"method": method, "args": _json_arg(args)}
        v = self._op("intents", app, body=body, confirm=_confirm(passkey))
        return _call_result(v)

    @deprecated("apps.view")
    def view(
        self,
        app: str,
        method: str,
        args: Any = None,
        stale: bool = False,
        min_index: int = 0,
    ) -> Any:
        """Run a read-only method. Strong by default; `stale=True` reads any
        replica at least as new as `min_index`."""
        body: Dict[str, Any] = {"method": method, "args": _json_arg(args)}
        if stale:
            body.update(consistency="stale", minIndex=min_index)
        else:
            body["consistency"] = "strong"
        v = self._op("views", app, body=body)
        if not isinstance(v, dict) or not isinstance(v.get("index"), int):
            raise CloudError("the node's answer is not a view", 0, "TRANSPORT", "null")
        return {
            "index": v["index"],
            "result": v.get("result"),
            "receipt": v.get("receipt"),
        }

    @deprecated("apps.list")
    def list_apps(self) -> Any:
        """The apps the caller can reach: `[{"appId", "name", "owner", "org", "bundle"}]`."""
        return (self._op("app_list") or {}).get("apps")

    @deprecated("apps.get")
    def get_app(self, app: str) -> Any:
        """One app: its directory record and state (`ready`, `initError`)."""
        return self._op("app_get", app)

    @deprecated("apps.access")
    def set_access(self, app: str, access: Any) -> None:
        """Who may reach an app: `{"kind": "owner" | "org" | "public"}`,
        `{"kind": "accounts", "accounts": [...]}` or `{"kind": "team", "team": id}`."""
        self._op("app_access", app, body={"access": access})

    @deprecated("apps.upgrade")
    def upgrade_app(
        self,
        app: str,
        bundle: Optional[str] = None,
        migrate: Optional[str] = None,
        version: Optional[str] = None,
        passkey: Any = None,
        package: Optional[str] = None,
        service: Optional[str] = None,
    ) -> Any:
        """Run `app` on another uploaded bundle (or `version` of its package).
        The node plans the migration from the two bundles' ABIs (`migrate`
        names an export to use instead). Where the app's owner approves
        upgrades, `passkey` is theirs: it answers the node's challenge over
        this exact upgrade. `package` and `service` move it to another package
        or service. A failed migration raises `CloudError` with code
        `UPGRADE_FAILED` (422).

        With an upgrade key in use (`create_api_key` with `upgrades`), pass a
        `version` of the key's package and no passkey: the node signs the
        upgrade with the key's own Ed25519 key in place of a passkey. A
        `bundle`, another package or another app is refused `403`."""
        body = {
            "bundle": bundle,
            "version": version,
            "migrate": migrate,
            "package": package,
            "service": service,
        }
        return self._op("app_upgrade", app, body=body, confirm=_confirm(passkey))

    @deprecated("apps.delete")
    def delete_app(self, app: str) -> None:
        """Delete an app (its owner's, or an org admin's)."""
        self._op("app_delete", app)

    @deprecated("apps.storage")
    def app_storage(self, app: str) -> Any:
        """What the app's storage holds: `{"keys", "bytes", "blobs", "blobBytes"}`."""
        return self._op("app_storage", app)

    @deprecated("apps.abi")
    def app_abi(self, app: str) -> Any:
        """The ABI the app's bundle embeds: `{"bundle", "abi"}`."""
        return self._op("app_abi", app)

    @deprecated("apps.metadata")
    def set_app_metadata(self, app: str, metadata: Any) -> None:
        """Set an app's metadata (its owner's, or an org admin's); `None` clears it."""
        self._op("app_metadata", app, body={"metadata": metadata})

    # ------------------------------------------------------------ blobs (routes)

    @deprecated("apps.blob_upload")
    def upload_blob(self, app: str, data: bytes) -> Any:
        """Upload a blob for `app`. Returns `{"blobId", "id", "size"}` (`blobId`
        base58, as apps print it; `id` hex)."""
        return self._op("blob_upload", app, body=data)

    @deprecated("apps.blob_download")
    def download_blob(self, app: str, id: str) -> bytes:
        """One of `app`'s blobs, by id (base58 or hex)."""
        return self._op("blob_download", app, id)

    @deprecated("apps.blob_list")
    def list_blobs(self, app: str) -> Any:
        """`app`'s blobs: `{"blobs": [{"blobId", "id", "size"}, ...]}`."""
        return self._op("blob_list", app)

    @deprecated("apps.blob_info")
    def blob_info(self, app: str, id: str) -> Any:
        """One blob's id and size."""
        return self._op("blob_info", app, id)

    @deprecated("apps.blob_delete")
    def delete_blob(self, app: str, id: str) -> Any:
        """Delete one of `app`'s blobs (its owner's or an org admin's)."""
        return self._op("blob_delete", app, id)

    # ------------------------------------------------------------ organizations (routes)

    @deprecated("orgs.create")
    def create_org(self, name: str, recovery_key: Optional[str] = None) -> str:
        """Create an organization; the caller owns it. Returns its id.
        `recovery_key` (`recovery_public_key(seed)`) lets whoever holds the
        seed recover it if its owner loses every passkey."""
        body = {"name": name}
        if recovery_key is not None:
            body["recoveryKey"] = recovery_key
        return _field(self._op("org_create", body=body), "orgId")

    @deprecated("orgs.get")
    def get_org(self, org: str) -> Any:
        """An organization: members, roles and the teams the caller can see."""
        return self._op("org_get", org)

    @deprecated("orgs.members")
    def org_members_security(self, org: str) -> Any:
        """Each member's lockout risk, for the organization's owner and admins
        (`GET /orgs/{org}/members?detail=security`, core #4902): a list of
        `{account, name, role, teams, passkeys, recoveryCode, lastActiveDay,
        sessions}`. `passkeys` is `1`, `2` or `"3+"`; `lastActiveDay` the UTC
        day (`YYYY-MM-DD`) of the member's latest sign-in or session exchange,
        `None` when unknown. Never a credential id, a passkey label, a session
        or an activity entry. A member below admin gets `403`, someone outside
        the organization `404 NOT_FOUND`; the admin's own session on the
        cluster's origin only. A node without the route answers a bare `404`
        or `405`."""
        return (self._op("org_members", org, query={"detail": "security"}) or {}).get(
            "members"
        )

    @deprecated("orgs.set_member")
    def set_member(self, org: str, account: str, role: str, passkey: Any) -> None:
        """Add `account` to the organization, or change its role
        (`guest`, `member`, `admin`, `owner`), approved with `passkey`."""
        body = {"account": account, "role": role}
        self._op("org_set_member", org, body=body, confirm=_confirm(passkey))

    @deprecated("orgs.remove_member")
    def remove_member(self, org: str, account: str, passkey: Any) -> None:
        """Remove `account` from the organization (yourself, to leave it),
        approved with `passkey`."""
        self._op(
            "org_remove_member",
            org,
            account,
            body={},
            confirm=_confirm(passkey),
        )

    @deprecated("orgs.capabilities")
    def set_capabilities(
        self,
        org: str,
        capabilities: int,
        passkey: Any,
        account: Optional[str] = None,
        team: Optional[str] = None,
    ) -> None:
        """Set capability bits (v1's `MemberCapabilities`): for `account` (in
        `team`, if given), or, with no account, the organization's default;
        approved with `passkey`."""
        body = {"capabilities": capabilities, "account": account, "team": team}
        self._op("org_capabilities", org, body=body, confirm=_confirm(passkey))

    @deprecated("orgs.team_create")
    def create_team(
        self, org: str, name: str, join: str = "invite", parent: Optional[str] = None
    ) -> str:
        """Create a team: `join="everyone"` (open: whoever reaches the parent is a
        member) or `"invite"` (restricted), under `parent` or at the top.
        Returns its id."""
        body = {"name": name, "join": join, "parent": parent}
        return _field(self._op("team_create", org, body=body), "teamId")

    @deprecated("orgs.app_team_list")
    def list_app_teams(self, org: str) -> Any:
        """The organization's mode (`legacy`, `standalone`, `shared`) and its app
        teams (one per package): `{"mode", "appTeams"}`. A guest sees only
        the app teams they are in."""
        return self._op("app_team_list", org)

    @deprecated("orgs.app_teams_scope")
    def scope_apps(
        self,
        org: str,
        passkey: Any,
        apps: Any = None,
        shared: Optional[bool] = None,
    ) -> Any:
        """Give apps their own access lists in one step (owner or admin),
        approved with `passkey`. `apps`: a list of `{"package", "name"?,
        "instances"?, "join"?, "keep"?}`; absent, every package with an
        organization-access instance. A package's `join` (`"invite"` closes
        its app team, `"everyone"` opens it) changes that team alone; a close
        keeps everyone who reaches the app, or, with `keep` (account ids),
        only the team's leads and those accounts. `shared=True` closes every
        app team. Returns `{"mode", "teams"}`."""
        # The node refuses unknown fields and reads an absent one as its
        # default, so only the fields given go.
        body: Dict[str, Any] = {}
        if apps is not None:
            body["apps"] = apps
        if shared is not None:
            body["shared"] = shared
        return self._op("app_teams_scope", org, body=body, confirm=_confirm(passkey))

    @deprecated("orgs.team_get")
    def get_team(self, org: str, team: str) -> Any:
        """A team: its members, sub-teams and apps."""
        return self._op("team_get", org, team)

    @deprecated("orgs.team_update")
    def update_team(
        self,
        org: str,
        team: str,
        name: Optional[str] = None,
        join: Optional[str] = None,
        metadata: Any = None,
        parent: Optional[str] = None,
        to_top: bool = False,
    ) -> None:
        """Rename, change how it is joined, set its metadata, or move it: under
        `parent`, or to the top with `to_top=True`. Unset arguments stay as they are."""
        if to_top and parent is not None:
            raise ValueError("pass parent or to_top, not both")
        body: Dict[str, Any] = {}
        if name is not None:
            body["name"] = name
        if join is not None:
            body["join"] = join
        if metadata is not None:
            body["metadata"] = metadata
        if parent is not None:
            body["parent"] = parent
        elif to_top:
            body["parent"] = None
        self._op("team_update", org, team, body=body)

    @deprecated("orgs.team_delete")
    def delete_team(self, org: str, team: str, passkey: Any = None) -> None:
        """Delete a team, the teams below it and their apps. When any of those
        teams holds apps the node asks for `passkey` (`412 CONFIRM_REQUIRED`,
        its details listing the `apps` that go, which the approval names);
        without one that `412` is raised. A team with no apps needs none."""
        self._op("team_delete", org, team, body={}, confirm=_confirm(passkey))

    @deprecated("orgs.team_set_member")
    def set_team_member(
        self,
        org: str,
        team: str,
        account: str,
        passkey: Any,
        role: Optional[str] = None,
        capabilities: Optional[int] = None,
    ) -> None:
        """Add `account` to a team, or change its row, approved with `passkey`:
        `role` is `lead`, `member` or `readOnly`; `capabilities` overrides the
        organization's default."""
        body = {"account": account, "role": role, "capabilities": capabilities}
        self._op("team_set_member", org, team, body=body, confirm=_confirm(passkey))

    @deprecated("orgs.team_remove_member")
    def remove_team_member(
        self, org: str, team: str, account: str, passkey: Any
    ) -> None:
        """Remove `account` from a team, approved with `passkey`. It does not
        come back through an open parent until it is added again."""
        self._op(
            "team_remove_member",
            org,
            team,
            account,
            body={},
            confirm=_confirm(passkey),
        )

    @deprecated("orgs.member_set_teams")
    def set_team_members(
        self, org: str, account: str, teams: Any, passkey: Any
    ) -> None:
        """Set `account`'s row in several teams of `org` at once (core `POST
        /orgs/{org}/members/{account}/teams`), approved with one `passkey`:
        Calimero Home's "give all apps". `teams`: a list of team ids (a plain
        member row) or `{"team", "role"?, "capabilities"?}` dicts, at most 64,
        each named once. Each team is judged as `set_team_member` would judge
        it, and nothing is set unless every one may be. A node from before the
        route answers a bare `404`."""
        if not isinstance(teams, (list, tuple)):
            raise ValueError("teams is a list")
        rows = [{"team": t} if isinstance(t, str) else t for t in teams]
        self._op(
            "member_set_teams",
            org,
            account,
            body={"teams": rows},
            confirm=_confirm(passkey),
        )

    @deprecated("orgs.member_remove_teams")
    def remove_team_members(
        self, org: str, account: str, teams: List[str], passkey: Any
    ) -> None:
        """Take `account` out of several teams of `org` at once, or leave them
        (`account` being the caller), approved with one `passkey` (core
        `DELETE /orgs/{org}/members/{account}/teams`): each judged as
        `remove_team_member` would judge it, the teams below included, and
        nothing taken unless every one may be."""
        self._op(
            "member_remove_teams",
            org,
            account,
            body={"teams": list(teams)},
            confirm=_confirm(passkey),
        )

    @deprecated("orgs.invite_create")
    def invite(
        self,
        org: str,
        role: str,
        passkey: Any,
        teams: Optional[List[str]] = None,
        ttl_secs: Optional[int] = None,
        uses: Optional[int] = None,
    ) -> str:
        """Invite someone into the organization with `role`, and into `teams`,
        approved with `passkey`. Returns the token to hand them (valid
        `ttl_secs`, default a week, for `uses` redemptions, default one)."""
        body = {
            "role": role,
            "teams": list(teams or []),
            "ttlSecs": ttl_secs,
            "uses": uses,
        }
        v = self._op("invite_create", org, body=body, confirm=_confirm(passkey))
        return _field(v, "token")

    @deprecated("orgs.invite_redeem")
    def redeem_invite(self, token: str) -> Any:
        """Join an organization (and its invited teams) with an invite token."""
        return self._op("invite_redeem", token)

    @deprecated("orgs.delete")
    def delete_org(self, org: str, passkey: Any) -> None:
        """Delete an organization with its teams, invites and apps: the owner's,
        approved with their `passkey`."""
        self._op("org_delete", org, body={}, confirm=_confirm(passkey))

    @deprecated("orgs.metadata")
    def set_org_metadata(self, org: str, metadata: Any) -> None:
        """Set an organization's metadata (an admin's, or a holder of
        `MANAGE_METADATA`); `None` clears it."""
        self._op("org_metadata", org, body={"metadata": metadata})

    @deprecated("orgs.member_metadata")
    def set_member_metadata(self, org: str, account: str, metadata: Any) -> None:
        """Set a member's metadata (their own, or an admin's); `None` clears it."""
        self._op(
            "member_metadata",
            org,
            account,
            body={"metadata": metadata},
        )

    # ------------------------------------------------------------ organization recovery (routes)

    @deprecated("orgs.recovery_set")
    def set_recovery_key(
        self, org: str, public_key: Optional[str], passkey: Any
    ) -> None:
        """Set or rotate (`public_key`) or remove (`None`) the organization's
        recovery key: its owner's, approved with `passkey`. A recovery under
        way is cancelled, and the old seed is refused from then on."""
        self._op(
            "recovery_set",
            org,
            body={"publicKey": public_key},
            confirm=_confirm(passkey),
        )

    @deprecated("orgs.recovery_cancel")
    def cancel_recovery(self, org: str, passkey: Any) -> None:
        """Cancel the recovery under way: the owner's, approved with `passkey`."""
        self._op("recovery_cancel", org, body={}, confirm=_confirm(passkey))

    @deprecated("orgs.recovery_finish")
    def finish_recovery(self, org: str, passkey: Any) -> Any:
        """Finish a recovery this account started, once its delay passed
        (`409 RECOVERY_NOT_READY` before), approved with this account's
        `passkey`: it becomes the owner and the previous owner leaves the
        organization. Answers `{ org, previousOwners }`."""
        return self._op("recovery_finish", org, body={}, confirm=_confirm(passkey))

    # ------------------------------------------------------------ governance (routes)

    @deprecated("auth.account_release_approval / orgs.release_approval")
    def approve_release(
        self, version: str, passkey: Any, org: Optional[str] = None
    ) -> Any:
        """Approve the mero-tee release `version` for `org`'s apps (or, without
        `org`, the account's own), with the owner's `passkey`."""
        body = {"version": version}
        if org is None:
            return self._op(
                "account_release_approval",
                body=body,
                confirm=_confirm(passkey),
            )
        return self._op(
            "org_release_approval",
            org,
            body=body,
            confirm=_confirm(passkey),
        )

    @deprecated("auth.account_outbound_approval / orgs.outbound_approval")
    def approve_outbound(
        self,
        package: str,
        passkey: Any,
        org: Optional[str] = None,
        prefixes: Optional[List[str]] = None,
        replace: bool = False,
    ) -> Any:
        """Approve URL prefixes `package`'s publisher may call out to, for `org`'s
        apps (or the account's own): every prefix its manifest declares when
        `prefixes` is `None`; `replace=True` sets exactly these (`[]` revokes)."""
        body = {
            "package": package,
            "prefixes": "manifest" if prefixes is None else list(prefixes),
            "replace": replace,
        }
        if org is None:
            return self._op(
                "account_outbound_approval",
                body=body,
                confirm=_confirm(passkey),
            )
        return self._op(
            "org_outbound_approval",
            org,
            body=body,
            confirm=_confirm(passkey),
        )

    @deprecated("orgs.upgrade")
    def upgrade_org(
        self,
        org: str,
        package: str,
        version: str,
        passkey: Any,
        team: Optional[str] = None,
        cascade: bool = False,
        migrate: Optional[str] = None,
    ) -> Any:
        """Upgrade every app of `org` running `package` to `version` (optionally
        only `team`, with `cascade` its subtree; `migrate` names the migration
        export), approved with the owner's `passkey`. Returns the run
        (`{"runId", ...}`)."""
        body = {
            "package": package,
            "version": version,
            "team": team,
            "cascade": cascade,
            "migrate": migrate,
        }
        return self._op("org_upgrade", org, body=body, confirm=_confirm(passkey))

    @deprecated("orgs.upgrade_status")
    def upgrade_run(self, org: str, run: str) -> Any:
        """An upgrade run's status and per-app results."""
        return self._op("org_upgrade_status", org, run)

    @deprecated("orgs.upgrade_retry")
    def retry_upgrade_run(self, org: str, run: str, passkey: Any) -> Any:
        """Run the failed apps of an upgrade run again, as a new run."""
        return self._op(
            "org_upgrade_retry", org, run, body={}, confirm=_confirm(passkey)
        )

    # ------------------------------------------------------------ packages (routes)

    @deprecated("packages.upload")
    def upload_package(self, mpk: bytes) -> Any:
        """Install a signed `.mpk` package. Returns what the node installed."""
        return self._op("package_upload", body=mpk)

    @deprecated("packages.install")
    def install_package(self, package: str, version: str) -> Any:
        """Install `package@version` from the node's package registry."""
        body = {"package": package, "version": version}
        return self._op("package_install", body=body)

    @deprecated("packages.list")
    def packages(self) -> Any:
        """Installed packages and their versions."""
        return (self._op("package_list") or {}).get("packages")

    @deprecated("packages.get")
    def package(self, package: str) -> Any:
        """One installed package and its versions."""
        return self._op("package_get", package)

    @deprecated("packages.uninstall / packages.version_uninstall")
    def uninstall_package(self, package: str, version: Optional[str] = None) -> Any:
        """Uninstall one version of a package, or (without `version`) all of it."""
        if version is None:
            return self._op("package_uninstall", package)
        return self._op("package_version_uninstall", package, version)

    @deprecated("packages.origins_list")
    def list_package_origins(self, package: str) -> Any:
        """The web origins registered for `package`: `{"package": ...,
        "origins": [{"origin", "rpId", "addedBy", "addedAt", "served"}, ...]}`
        (`served` absent from a node that predates served origins)."""
        return self._op("origins_list", package)

    @deprecated("packages.origins_add")
    def add_package_origin(
        self,
        package: str,
        origin: str,
        passkey: Any,
        rp_id: Optional[str] = None,
        served: bool = False,
    ) -> Any:
        """Register `origin` (`https://app.example`) for `package`'s frontends,
        approved with `passkey`. `rp_id` is the WebAuthn RP ID passkeys from
        that origin are made for; without it the node derives one from the
        origin. Returns `{"package", "origin", "rpId", "addedBy", "addedAt",
        "served"}`.

        `served=True` registers a custom domain: the cluster also serves the
        package's frontend on it (`https://host`, no port; not a name under
        the apps domain or the cluster's rp-id). The node refuses it `400
        NOT_SERVING` on a cluster that serves no frontends, `409
        CLUSTER_NOT_UPGRADED` while a node runs an older release, `409
        ORIGIN_TAKEN` when the origin is held with the other `served`, and
        `429 QUOTA_EXCEEDED` past 4 per package or 64 per cluster."""
        return self._op(
            "origins_add",
            package,
            body=_origin_body(origin, rp_id, served),
            confirm=_confirm(passkey),
        )

    @deprecated("packages.origins_remove")
    def remove_package_origin(self, package: str, origin: str, passkey: Any) -> Any:
        """Remove `origin` from `package`, approved with `passkey`. Returns
        `{"package", "origin", "removed": true}`."""
        return self._op(
            "origins_remove",
            package,
            body={"origin": origin},
            confirm=_confirm(passkey),
        )

    # ------------------------------------------------------------ the Ops API (routes)
    #
    # The operator's calls, authenticated with the node's ops token:
    # `client.use_api_key(ops_token)` first. The Ops API has no read path
    # into apps' data: counts, placement, releases, packages, quotas,
    # backups and metrics only. Its decisions are signed with the operator
    # key (`use_operator_key`), else the node's `428` is raised.

    @deprecated("ops.status")
    def ops_status(self) -> Any:
        """Node, group and directory counts: `{"node", "system", "governance",
        "groups", "nodes", "apps", "accounts"}`."""
        return self._op("ops_status")

    @deprecated("ops.metrics")
    def ops_metrics(self) -> str:
        """Prometheus metrics, as text."""
        return self._op("ops_metrics")

    @deprecated("ops.register_node")
    def ops_register_node(self, node_id: int, addr: str, zone: str = "") -> None:
        """Register node `node_id` at `addr` (its URL) in the directory, signed
        with the operator key. A node started without `--bootstrap` and
        outside the genesis `--peers` then joins the system group as a
        learner, and replica moves may place apps on it."""
        body = {"id": node_id, "addr": addr, "zone": zone}
        self._op("ops_register_node", body=body)

    @deprecated("ops.remove_node")
    def ops_remove_node(self, node_id: int) -> Any:
        """Remove node `node_id` from the cluster once no app keeps a replica on
        it, signed with the operator key. Runs on the system leader; a
        `NOT_LEADER` answer is followed there.
        Returns the system group's status."""
        return self._op(
            "ops_remove_node",
            str(node_id),
            body={},
        )

    @deprecated("ops.set_replicas")
    def ops_set_replicas(self, app: str, replicas: List[int]) -> Any:
        """Move `app` to the replica node ids `replicas` (at least 3), signed
        with the operator key. Runs on the app group's leader; a `NOT_LEADER` answer is followed there.
        Returns the group's status."""
        return self._op(
            "ops_set_replicas",
            app,
            body={"replicas": list(replicas)},
        )

    @deprecated("ops.backup")
    def ops_backup(self) -> int:
        """Write encrypted snapshots of the apps this node leads (to the node's
        `--backup-dir`). Returns how many it wrote."""
        written = (self._op("ops_backup") or {}).get("written")
        return written if isinstance(written, int) and written >= 0 else 0

    @deprecated("ops.quotas_get")
    def ops_quotas(self) -> Any:
        """The per-account quotas."""
        return self._op("ops_quotas_get")

    @deprecated("ops.quotas_set")
    def ops_set_quotas(self, quotas: Any) -> Any:
        """Change the per-account limits `quotas` names, signed with the
        operator key; the others stay as they are. A key that names no limit is refused (`UNKNOWN_QUOTA`).
        """
        return self._op(
            "ops_quotas_set",
            body=_json_arg(quotas),
        )

    @deprecated("ops.release")
    def ops_approve_release(self, version: str, withdraw: bool = False) -> Any:
        """Approve (or, `withdraw=True`, withdraw) the mero-tee release `version`
        for every app, where the deployment gave the operator release
        approvals (`merod --release-approvals operator`)."""
        body = {"version": version, "approved": not withdraw}
        return self._op("ops_release", body=body)

    @deprecated("ops.outbound")
    def ops_approve_outbound(
        self, package: str, prefixes: Optional[List[str]] = None, replace: bool = False
    ) -> Any:
        """Approve URL prefixes `package`'s publisher may call out to, for every
        app, where the operator holds outbound approvals: every prefix its
        manifest declares when `prefixes` is `None`; `replace=True` sets
        exactly these (`[]` revokes)."""
        body = {
            "package": package,
            "prefixes": "manifest" if prefixes is None else list(prefixes),
            "replace": replace,
        }
        return self._op("ops_outbound", body=body)

    @deprecated("ops.upgrade")
    def ops_upgrade_app(
        self,
        app: str,
        bundle: Optional[str] = None,
        version: Optional[str] = None,
        package: Optional[str] = None,
        service: Optional[str] = None,
        migrate: Optional[str] = None,
    ) -> Any:
        """Upgrade `app` (to an uploaded `bundle`, or `version` of its package,
        optionally moving it to `package`/`service`; `migrate` names the
        migration export), where the operator holds app upgrades."""
        body = {
            "bundle": bundle,
            "version": version,
            "package": package,
            "service": service,
            "migrate": migrate,
        }
        return self._op("ops_upgrade", app, body=body)

    @deprecated("ops.org_upgrade")
    def ops_upgrade_org(
        self,
        org: str,
        package: str,
        version: str,
        team: Optional[str] = None,
        cascade: bool = False,
        migrate: Optional[str] = None,
    ) -> Any:
        """Upgrade every app of `org` running `package` to `version` (optionally
        only `team`, with `cascade` its subtree), where the operator holds app
        upgrades. Returns the run, which `upgrade_run` follows."""
        body = {
            "package": package,
            "version": version,
            "team": team,
            "cascade": cascade,
            "migrate": migrate,
        }
        return self._op("ops_org_upgrade", org, body=body)

    @deprecated("ops.served")
    def ops_served_origins(self) -> Any:
        """Every served origin (custom domain) the cluster's log holds: `{"fresh",
        "origins": [{"origin", "host", "label", "addedAt"}, ...]}`, with the
        package's app-host `label` and no package name or account."""
        return self._op("ops_served")

    @deprecated("ops.package_list")
    def ops_packages(self) -> Any:
        """The packages the operator installed."""
        return self._op("ops_package_list")

    @deprecated("ops.package_upload")
    def ops_upload_package(self, mpk: bytes) -> Any:
        """Install a signed `.mpk` for the operator's upgrades."""
        return self._op("ops_package_upload", body=mpk)

    @deprecated("ops.package_install")
    def ops_install_package(self, package: str, version: str) -> Any:
        """Install `package@version` from the registry for the operator's upgrades."""
        body = {"package": package, "version": version}
        return self._op("ops_package_install", body=body)

    @deprecated("ops.package_uninstall / ops.package_version_uninstall")
    def ops_uninstall_package(self, package: str, version: Optional[str] = None) -> Any:
        """Uninstall a package the operator installed (or one `version` of it)."""
        if version is None:
            return self._op("ops_package_uninstall", package)
        return self._op("ops_package_version_uninstall", package, version)


def _call_result(v: Any) -> Dict[str, Any]:
    """A call's answer as earlier releases gave it (core's `CallResult`)."""
    if not isinstance(v, dict) or not isinstance(v.get("index"), int):
        raise CloudError("the node's answer is not a call", 0, "TRANSPORT", "null")
    return {
        "index": v["index"],
        "result": v.get("result"),
        "events": v.get("events") or [],
        "receipt": v.get("receipt"),
    }


def _origin_body(origin: str, rp_id: Optional[str], served: bool) -> Dict[str, Any]:
    """`{"origin": ...}`, with `rpId` only when the caller names one (the node
    resolves it otherwise, and signs over the one it resolved), and `served:
    true` only for a served origin: a node that predates served origins
    refuses unknown fields, and takes a plain add unchanged."""
    body: Dict[str, Any] = {"origin": origin}
    if rp_id is not None:
        body["rpId"] = rp_id
    if served:
        body["served"] = True
    return body


# The facade's groups: `client.orgs.get(org=...)`. `packages` was a method,
# and stays one when called.
for _name in facade.GROUPS:
    _legacy = Client.__dict__.get(_name)
    setattr(Client, _name, _GroupAttribute(_name, _legacy))
del _name, _legacy

# The name earlier releases used; one class, two names.
CloudClient = Client
