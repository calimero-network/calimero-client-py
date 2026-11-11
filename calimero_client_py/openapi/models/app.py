from collections.abc import Mapping
from typing import Any, TypeVar, Optional, BinaryIO, TextIO, TYPE_CHECKING, Generator

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..types import UNSET, Unset
from typing import cast
from typing import cast, Union
from typing import Union

if TYPE_CHECKING:
    from ..models.access_type_0 import AccessType0
    from ..models.access_type_3 import AccessType3
    from ..models.access_type_1 import AccessType1
    from ..models.access_type_4 import AccessType4
    from ..models.access_type_2 import AccessType2


T = TypeVar("T", bound="App")


@_attrs_define
class App:
    """An app instance (a context) as the directory records it.

    Attributes:
        app_id (str):
        name (str):
        group (int):
        bundle (str):
        owner (str):
        org (Union[None, str]):
        access (Union['AccessType0', 'AccessType1', 'AccessType2', 'AccessType3', 'AccessType4']): Who may reach an app
            at all.
        confirm_methods (list[str]):
        replicas (list[int]):
        created_at (int):
        init_args (Any):
        metadata (Any):
        compute_bundle (Union[Unset, str]):
        last_upgrade (Union[Unset, Any]):
        release (Union[Unset, Any]):
    """

    app_id: str
    name: str
    group: int
    bundle: str
    owner: str
    org: Union[None, str]
    access: Union[
        "AccessType0", "AccessType1", "AccessType2", "AccessType3", "AccessType4"
    ]
    confirm_methods: list[str]
    replicas: list[int]
    created_at: int
    init_args: Any
    metadata: Any
    compute_bundle: Union[Unset, str] = UNSET
    last_upgrade: Union[Unset, Any] = UNSET
    release: Union[Unset, Any] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.access_type_0 import AccessType0
        from ..models.access_type_3 import AccessType3
        from ..models.access_type_1 import AccessType1
        from ..models.access_type_4 import AccessType4
        from ..models.access_type_2 import AccessType2

        app_id = self.app_id

        name = self.name

        group = self.group

        bundle = self.bundle

        owner = self.owner

        org: Union[None, str]
        org = self.org

        access: dict[str, Any]
        if isinstance(self.access, AccessType0):
            access = self.access.to_dict()
        elif isinstance(self.access, AccessType1):
            access = self.access.to_dict()
        elif isinstance(self.access, AccessType2):
            access = self.access.to_dict()
        elif isinstance(self.access, AccessType3):
            access = self.access.to_dict()
        else:
            access = self.access.to_dict()

        confirm_methods = self.confirm_methods

        replicas = self.replicas

        created_at = self.created_at

        init_args = self.init_args

        metadata = self.metadata

        compute_bundle = self.compute_bundle

        last_upgrade = self.last_upgrade

        release = self.release

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "appId": app_id,
                "name": name,
                "group": group,
                "bundle": bundle,
                "owner": owner,
                "org": org,
                "access": access,
                "confirmMethods": confirm_methods,
                "replicas": replicas,
                "createdAt": created_at,
                "initArgs": init_args,
                "metadata": metadata,
            }
        )
        if compute_bundle is not UNSET:
            field_dict["computeBundle"] = compute_bundle
        if last_upgrade is not UNSET:
            field_dict["lastUpgrade"] = last_upgrade
        if release is not UNSET:
            field_dict["release"] = release

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.access_type_0 import AccessType0
        from ..models.access_type_3 import AccessType3
        from ..models.access_type_1 import AccessType1
        from ..models.access_type_4 import AccessType4
        from ..models.access_type_2 import AccessType2

        d = dict(src_dict)
        app_id = d.pop("appId")

        name = d.pop("name")

        group = d.pop("group")

        bundle = d.pop("bundle")

        owner = d.pop("owner")

        def _parse_org(data: object) -> Union[None, str]:
            if data is None:
                return data
            return cast(Union[None, str], data)

        org = _parse_org(d.pop("org"))

        def _parse_access(
            data: object,
        ) -> Union[
            "AccessType0", "AccessType1", "AccessType2", "AccessType3", "AccessType4"
        ]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_access_type_0 = AccessType0.from_dict(data)

                return componentsschemas_access_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_access_type_1 = AccessType1.from_dict(data)

                return componentsschemas_access_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_access_type_2 = AccessType2.from_dict(data)

                return componentsschemas_access_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_access_type_3 = AccessType3.from_dict(data)

                return componentsschemas_access_type_3
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_access_type_4 = AccessType4.from_dict(data)

            return componentsschemas_access_type_4

        access = _parse_access(d.pop("access"))

        confirm_methods = cast(list[str], d.pop("confirmMethods"))

        replicas = cast(list[int], d.pop("replicas"))

        created_at = d.pop("createdAt")

        init_args = d.pop("initArgs")

        metadata = d.pop("metadata")

        compute_bundle = d.pop("computeBundle", UNSET)

        last_upgrade = d.pop("lastUpgrade", UNSET)

        release = d.pop("release", UNSET)

        app = cls(
            app_id=app_id,
            name=name,
            group=group,
            bundle=bundle,
            owner=owner,
            org=org,
            access=access,
            confirm_methods=confirm_methods,
            replicas=replicas,
            created_at=created_at,
            init_args=init_args,
            metadata=metadata,
            compute_bundle=compute_bundle,
            last_upgrade=last_upgrade,
            release=release,
        )

        app.additional_properties = d
        return app

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
