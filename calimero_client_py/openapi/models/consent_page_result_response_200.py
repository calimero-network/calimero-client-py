from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..models.consent_page_result_response_200_kind import (
    ConsentPageResultResponse200Kind,
)
from ..models.consent_page_result_response_200_status import (
    ConsentPageResultResponse200Status,
)
from typing import cast, Union

T = TypeVar("T", bound="ConsentPageResultResponse200")


@_attrs_define
class ConsentPageResultResponse200:
    """
    Attributes:
        request_id (str):
        kind (ConsentPageResultResponse200Kind):
        status (ConsentPageResultResponse200Status):
        decided_at (Union[None, int]):
    """

    request_id: str
    kind: ConsentPageResultResponse200Kind
    status: ConsentPageResultResponse200Status
    decided_at: Union[None, int]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        request_id = self.request_id

        kind = self.kind.value

        status = self.status.value

        decided_at: Union[None, int]
        decided_at = self.decided_at

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "requestId": request_id,
                "kind": kind,
                "status": status,
                "decidedAt": decided_at,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        request_id = d.pop("requestId")

        kind = ConsentPageResultResponse200Kind(d.pop("kind"))

        status = ConsentPageResultResponse200Status(d.pop("status"))

        def _parse_decided_at(data: object) -> Union[None, int]:
            if data is None:
                return data
            return cast(Union[None, int], data)

        decided_at = _parse_decided_at(d.pop("decidedAt"))

        consent_page_result_response_200 = cls(
            request_id=request_id,
            kind=kind,
            status=status,
            decided_at=decided_at,
        )

        consent_page_result_response_200.additional_properties = d
        return consent_page_result_response_200

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
