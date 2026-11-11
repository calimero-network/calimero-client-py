from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast

if TYPE_CHECKING:
    from ..models.app_usage_response_200_days_item import AppUsageResponse200DaysItem


T = TypeVar("T", bound="AppUsageResponse200")


@_attrs_define
class AppUsageResponse200:
    """
    Attributes:
        today (Any):
        days (list['AppUsageResponse200DaysItem']):
    """

    today: Any
    days: list["AppUsageResponse200DaysItem"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.app_usage_response_200_days_item import (
            AppUsageResponse200DaysItem,
        )

        today = self.today

        days = []
        for days_item_data in self.days:
            days_item = days_item_data.to_dict()
            days.append(days_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "today": today,
                "days": days,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.app_usage_response_200_days_item import (
            AppUsageResponse200DaysItem,
        )

        d = dict(src_dict)
        today = d.pop("today")

        days = []
        _days = d.pop("days")
        for days_item_data in _days:
            days_item = AppUsageResponse200DaysItem.from_dict(days_item_data)

            days.append(days_item)

        app_usage_response_200 = cls(
            today=today,
            days=days,
        )

        app_usage_response_200.additional_properties = d
        return app_usage_response_200

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
