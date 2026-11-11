from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast

if TYPE_CHECKING:
    from ..models.org_outbound_approvals_additional_property import (
        OrgOutboundApprovalsAdditionalProperty,
    )


T = TypeVar("T", bound="OrgOutboundApprovals")


@_attrs_define
class OrgOutboundApprovals:
    """ """

    additional_properties: dict[str, "OrgOutboundApprovalsAdditionalProperty"] = (
        _attrs_field(init=False, factory=dict)
    )

    def to_dict(self) -> dict[str, Any]:
        from ..models.org_outbound_approvals_additional_property import (
            OrgOutboundApprovalsAdditionalProperty,
        )

        field_dict: dict[str, Any] = {}
        for prop_name, prop in self.additional_properties.items():
            field_dict[prop_name] = prop.to_dict()

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.org_outbound_approvals_additional_property import (
            OrgOutboundApprovalsAdditionalProperty,
        )

        d = dict(src_dict)
        org_outbound_approvals = cls()

        additional_properties = {}
        for prop_name, prop_dict in d.items():
            additional_property = OrgOutboundApprovalsAdditionalProperty.from_dict(
                prop_dict
            )

            additional_properties[prop_name] = additional_property

        org_outbound_approvals.additional_properties = additional_properties
        return org_outbound_approvals

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> "OrgOutboundApprovalsAdditionalProperty":
        return self.additional_properties[key]

    def __setitem__(
        self, key: str, value: "OrgOutboundApprovalsAdditionalProperty"
    ) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
