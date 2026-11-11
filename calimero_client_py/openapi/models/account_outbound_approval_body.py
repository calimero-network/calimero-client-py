from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..types import UNSET, Unset
from typing import cast
from typing import cast, Union
from typing import Literal, cast
from typing import Union

if TYPE_CHECKING:
    from ..models.confirm import Confirm


T = TypeVar("T", bound="AccountOutboundApprovalBody")


@_attrs_define
class AccountOutboundApprovalBody:
    """
    Attributes:
        package (str):
        prefixes (Union[Literal['manifest'], Unset, list[str]]): `manifest` (every prefix the release declares) or these
            prefixes.
        replace (Union[Unset, bool]):
        confirm (Union[Unset, Confirm]): A passkey assertion over the challenge a `412 CONFIRM_REQUIRED` named: the same
            request sent again with it approves exactly that request.
    """

    package: str
    prefixes: Union[Literal["manifest"], Unset, list[str]] = UNSET
    replace: Union[Unset, bool] = UNSET
    confirm: Union[Unset, "Confirm"] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.confirm import Confirm

        package = self.package

        prefixes: Union[Literal["manifest"], Unset, list[str]]
        if isinstance(self.prefixes, Unset):
            prefixes = UNSET
        elif isinstance(self.prefixes, list):
            prefixes = self.prefixes

        else:
            prefixes = self.prefixes

        replace = self.replace

        confirm: Union[Unset, dict[str, Any]] = UNSET
        if not isinstance(self.confirm, Unset):
            confirm = self.confirm.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "package": package,
            }
        )
        if prefixes is not UNSET:
            field_dict["prefixes"] = prefixes
        if replace is not UNSET:
            field_dict["replace"] = replace
        if confirm is not UNSET:
            field_dict["confirm"] = confirm

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.confirm import Confirm

        d = dict(src_dict)
        package = d.pop("package")

        def _parse_prefixes(
            data: object,
        ) -> Union[Literal["manifest"], Unset, list[str]]:
            if isinstance(data, Unset):
                return data
            componentsschemas_outbound_selection_type_0 = cast(
                Literal["manifest"], data
            )
            if componentsschemas_outbound_selection_type_0 != "manifest":
                raise ValueError(
                    f"/components/schemas/OutboundSelection_type_0 must match const 'manifest', got '{componentsschemas_outbound_selection_type_0}'"
                )
            return componentsschemas_outbound_selection_type_0
            if not isinstance(data, list):
                raise TypeError()
            componentsschemas_outbound_selection_type_1 = cast(list[str], data)

            return componentsschemas_outbound_selection_type_1

        prefixes = _parse_prefixes(d.pop("prefixes", UNSET))

        replace = d.pop("replace", UNSET)

        _confirm = d.pop("confirm", UNSET)
        confirm: Union[Unset, Confirm]
        if isinstance(_confirm, Unset):
            confirm = UNSET
        else:
            confirm = Confirm.from_dict(_confirm)

        account_outbound_approval_body = cls(
            package=package,
            prefixes=prefixes,
            replace=replace,
            confirm=confirm,
        )

        account_outbound_approval_body.additional_properties = d
        return account_outbound_approval_body

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
