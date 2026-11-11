from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="OrgGetResponse200")


@_attrs_define
class OrgGetResponse200:
    """
    Attributes:
        org_id (str):
        name (str):
        owner (str):
        members (Any):
        teams (Any):
        metadata (Any):
    """

    org_id: str
    name: str
    owner: str
    members: Any
    teams: Any
    metadata: Any
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        org_id = self.org_id

        name = self.name

        owner = self.owner

        members = self.members

        teams = self.teams

        metadata = self.metadata

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "orgId": org_id,
                "name": name,
                "owner": owner,
                "members": members,
                "teams": teams,
                "metadata": metadata,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        org_id = d.pop("orgId")

        name = d.pop("name")

        owner = d.pop("owner")

        members = d.pop("members")

        teams = d.pop("teams")

        metadata = d.pop("metadata")

        org_get_response_200 = cls(
            org_id=org_id,
            name=name,
            owner=owner,
            members=members,
            teams=teams,
            metadata=metadata,
        )

        org_get_response_200.additional_properties = d
        return org_get_response_200

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
