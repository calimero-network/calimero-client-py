from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="UsageResponse200")


@_attrs_define
class UsageResponse200:
    """
    Attributes:
        bundle_bytes (str): an unsigned 64-bit integer as a decimal string (D20)
        developer_fuel_today (str): an unsigned 64-bit integer as a decimal string (D20)
        developer_writes_today (str): an unsigned 64-bit integer as a decimal string (D20)
    """

    bundle_bytes: str
    developer_fuel_today: str
    developer_writes_today: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bundle_bytes = self.bundle_bytes

        developer_fuel_today = self.developer_fuel_today

        developer_writes_today = self.developer_writes_today

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "bundleBytes": bundle_bytes,
                "developerFuelToday": developer_fuel_today,
                "developerWritesToday": developer_writes_today,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        bundle_bytes = d.pop("bundleBytes")

        developer_fuel_today = d.pop("developerFuelToday")

        developer_writes_today = d.pop("developerWritesToday")

        usage_response_200 = cls(
            bundle_bytes=bundle_bytes,
            developer_fuel_today=developer_fuel_today,
            developer_writes_today=developer_writes_today,
        )

        usage_response_200.additional_properties = d
        return usage_response_200

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
