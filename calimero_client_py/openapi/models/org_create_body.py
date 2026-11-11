from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..types import UNSET, Unset
from typing import Union

T = TypeVar("T", bound="OrgCreateBody")


@_attrs_define
class OrgCreateBody:
    """
    Attributes:
        name (str):
        recovery_key (Union[Unset, str]):
    """

    name: str
    recovery_key: Union[Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        recovery_key = self.recovery_key

        field_dict: dict[str, Any] = {}

        field_dict.update(
            {
                "name": name,
            }
        )
        if recovery_key is not UNSET:
            field_dict["recoveryKey"] = recovery_key

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name")

        recovery_key = d.pop("recoveryKey", UNSET)

        org_create_body = cls(
            name=name,
            recovery_key=recovery_key,
        )

        return org_create_body
