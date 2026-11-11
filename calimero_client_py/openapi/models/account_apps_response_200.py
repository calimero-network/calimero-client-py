from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import cast, Union

T = TypeVar("T", bound="AccountAppsResponse200")


@_attrs_define
class AccountAppsResponse200:
    """
    Attributes:
        scope (Any):
        governance (Any):
        approved_releases (Any):
        cluster_releases (Any):
        apps (list[Any]):
        next_ (Union[None, str]):
    """

    scope: Any
    governance: Any
    approved_releases: Any
    cluster_releases: Any
    apps: list[Any]
    next_: Union[None, str]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        scope = self.scope

        governance = self.governance

        approved_releases = self.approved_releases

        cluster_releases = self.cluster_releases

        apps = self.apps

        next_: Union[None, str]
        next_ = self.next_

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "scope": scope,
                "governance": governance,
                "approvedReleases": approved_releases,
                "clusterReleases": cluster_releases,
                "apps": apps,
                "next": next_,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        scope = d.pop("scope")

        governance = d.pop("governance")

        approved_releases = d.pop("approvedReleases")

        cluster_releases = d.pop("clusterReleases")

        apps = cast(list[Any], d.pop("apps"))

        def _parse_next_(data: object) -> Union[None, str]:
            if data is None:
                return data
            return cast(Union[None, str], data)

        next_ = _parse_next_(d.pop("next"))

        account_apps_response_200 = cls(
            scope=scope,
            governance=governance,
            approved_releases=approved_releases,
            cluster_releases=cluster_releases,
            apps=apps,
            next_=next_,
        )

        account_apps_response_200.additional_properties = d
        return account_apps_response_200

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
