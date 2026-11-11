from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..models.me_response_200_via import MeResponse200Via
from typing import cast
from typing import cast, Union

if TYPE_CHECKING:
    from ..models.me_response_200_orgs_item import MeResponse200OrgsItem
    from ..models.me_response_200_passkeys_item import MeResponse200PasskeysItem


T = TypeVar("T", bound="MeResponse200")


@_attrs_define
class MeResponse200:
    """
    Attributes:
        account_id (str):
        name (str):
        passkeys (list['MeResponse200PasskeysItem']):
        orgs (list['MeResponse200OrgsItem']):
        via (MeResponse200Via):
        passkey_id (Union[None, str]):
        app (Union[None, str]):
        origin (Union[None, str]):
        approved_releases (Any):
        outbound_approvals (Any):
        recovery_code (Any):
        recovery_pending (Any):
    """

    account_id: str
    name: str
    passkeys: list["MeResponse200PasskeysItem"]
    orgs: list["MeResponse200OrgsItem"]
    via: MeResponse200Via
    passkey_id: Union[None, str]
    app: Union[None, str]
    origin: Union[None, str]
    approved_releases: Any
    outbound_approvals: Any
    recovery_code: Any
    recovery_pending: Any
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.me_response_200_orgs_item import MeResponse200OrgsItem
        from ..models.me_response_200_passkeys_item import MeResponse200PasskeysItem

        account_id = self.account_id

        name = self.name

        passkeys = []
        for passkeys_item_data in self.passkeys:
            passkeys_item = passkeys_item_data.to_dict()
            passkeys.append(passkeys_item)

        orgs = []
        for orgs_item_data in self.orgs:
            orgs_item = orgs_item_data.to_dict()
            orgs.append(orgs_item)

        via = self.via.value

        passkey_id: Union[None, str]
        passkey_id = self.passkey_id

        app: Union[None, str]
        app = self.app

        origin: Union[None, str]
        origin = self.origin

        approved_releases = self.approved_releases

        outbound_approvals = self.outbound_approvals

        recovery_code = self.recovery_code

        recovery_pending = self.recovery_pending

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "accountId": account_id,
                "name": name,
                "passkeys": passkeys,
                "orgs": orgs,
                "via": via,
                "passkeyId": passkey_id,
                "app": app,
                "origin": origin,
                "approvedReleases": approved_releases,
                "outboundApprovals": outbound_approvals,
                "recoveryCode": recovery_code,
                "recoveryPending": recovery_pending,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.me_response_200_orgs_item import MeResponse200OrgsItem
        from ..models.me_response_200_passkeys_item import MeResponse200PasskeysItem

        d = dict(src_dict)
        account_id = d.pop("accountId")

        name = d.pop("name")

        passkeys = []
        _passkeys = d.pop("passkeys")
        for passkeys_item_data in _passkeys:
            passkeys_item = MeResponse200PasskeysItem.from_dict(passkeys_item_data)

            passkeys.append(passkeys_item)

        orgs = []
        _orgs = d.pop("orgs")
        for orgs_item_data in _orgs:
            orgs_item = MeResponse200OrgsItem.from_dict(orgs_item_data)

            orgs.append(orgs_item)

        via = MeResponse200Via(d.pop("via"))

        def _parse_passkey_id(data: object) -> Union[None, str]:
            if data is None:
                return data
            return cast(Union[None, str], data)

        passkey_id = _parse_passkey_id(d.pop("passkeyId"))

        def _parse_app(data: object) -> Union[None, str]:
            if data is None:
                return data
            return cast(Union[None, str], data)

        app = _parse_app(d.pop("app"))

        def _parse_origin(data: object) -> Union[None, str]:
            if data is None:
                return data
            return cast(Union[None, str], data)

        origin = _parse_origin(d.pop("origin"))

        approved_releases = d.pop("approvedReleases")

        outbound_approvals = d.pop("outboundApprovals")

        recovery_code = d.pop("recoveryCode")

        recovery_pending = d.pop("recoveryPending")

        me_response_200 = cls(
            account_id=account_id,
            name=name,
            passkeys=passkeys,
            orgs=orgs,
            via=via,
            passkey_id=passkey_id,
            app=app,
            origin=origin,
            approved_releases=approved_releases,
            outbound_approvals=outbound_approvals,
            recovery_code=recovery_code,
            recovery_pending=recovery_pending,
        )

        me_response_200.additional_properties = d
        return me_response_200

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
