from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast

T = TypeVar("T", bound="AppXcallsResponse200")


@_attrs_define
class AppXcallsResponse200:
    """
    Attributes:
        pending (list[Any]):
        dead_letters (list[Any]):
    """

    pending: list[Any]
    dead_letters: list[Any]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        pending = self.pending

        dead_letters = self.dead_letters

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "pending": pending,
                "deadLetters": dead_letters,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        pending = cast(list[Any], d.pop("pending"))

        dead_letters = cast(list[Any], d.pop("deadLetters"))

        app_xcalls_response_200 = cls(
            pending=pending,
            dead_letters=dead_letters,
        )

        app_xcalls_response_200.additional_properties = d
        return app_xcalls_response_200

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
