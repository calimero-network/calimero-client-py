from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Literal, cast

if TYPE_CHECKING:
    from ..models.consent_page_create_body_type_1_subject import (
        ConsentPageCreateBodyType1Subject,
    )


T = TypeVar("T", bound="ConsentPageCreateBodyType1")


@_attrs_define
class ConsentPageCreateBodyType1:
    """
    Attributes:
        kind (Literal['upgrade']):
        subject (ConsentPageCreateBodyType1Subject):
        return_to (str):
    """

    kind: Literal["upgrade"]
    subject: "ConsentPageCreateBodyType1Subject"
    return_to: str

    def to_dict(self) -> dict[str, Any]:
        from ..models.consent_page_create_body_type_1_subject import (
            ConsentPageCreateBodyType1Subject,
        )

        kind = self.kind

        subject = self.subject.to_dict()

        return_to = self.return_to

        field_dict: dict[str, Any] = {}

        field_dict.update(
            {
                "kind": kind,
                "subject": subject,
                "returnTo": return_to,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.consent_page_create_body_type_1_subject import (
            ConsentPageCreateBodyType1Subject,
        )

        d = dict(src_dict)
        kind = cast(Literal["upgrade"], d.pop("kind"))
        if kind != "upgrade":
            raise ValueError(f"kind must match const 'upgrade', got '{kind}'")

        subject = ConsentPageCreateBodyType1Subject.from_dict(d.pop("subject"))

        return_to = d.pop("returnTo")

        consent_page_create_body_type_1 = cls(
            kind=kind,
            subject=subject,
            return_to=return_to,
        )

        return consent_page_create_body_type_1
