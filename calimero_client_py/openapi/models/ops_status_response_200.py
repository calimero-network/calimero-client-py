from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..types import UNSET, Unset
from typing import Union

T = TypeVar("T", bound="OpsStatusResponse200")


@_attrs_define
class OpsStatusResponse200:
    """
    Attributes:
        node (Any):
        system (Any):
        governance (Any):
        groups (Any):
        nodes (Any):
        apps (Any):
        accounts (Any):
        rollback (Union[Unset, Any]):
    """

    node: Any
    system: Any
    governance: Any
    groups: Any
    nodes: Any
    apps: Any
    accounts: Any
    rollback: Union[Unset, Any] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        node = self.node

        system = self.system

        governance = self.governance

        groups = self.groups

        nodes = self.nodes

        apps = self.apps

        accounts = self.accounts

        rollback = self.rollback

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "node": node,
                "system": system,
                "governance": governance,
                "groups": groups,
                "nodes": nodes,
                "apps": apps,
                "accounts": accounts,
            }
        )
        if rollback is not UNSET:
            field_dict["rollback"] = rollback

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        node = d.pop("node")

        system = d.pop("system")

        governance = d.pop("governance")

        groups = d.pop("groups")

        nodes = d.pop("nodes")

        apps = d.pop("apps")

        accounts = d.pop("accounts")

        rollback = d.pop("rollback", UNSET)

        ops_status_response_200 = cls(
            node=node,
            system=system,
            governance=governance,
            groups=groups,
            nodes=nodes,
            apps=apps,
            accounts=accounts,
            rollback=rollback,
        )

        ops_status_response_200.additional_properties = d
        return ops_status_response_200

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
