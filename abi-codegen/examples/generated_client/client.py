"""Generated client from WASM ABI schema"""

from typing import Dict, List, Any, Optional, Union
import json
from calimero import Client, ClientError

from .types import *


class ABIClient:
    """Generated client for WASM ABI methods"""

    def __init__(self, client: Client, context_id: str, executor_public_key: str):
        """Initialize with a Calimero client, context ID, and executor public key"""
        self.client = client
        self.context_id = context_id
        self.executor_public_key = executor_public_key

    def init(self) -> Any:
        """
        init method from ABI

        Returns:
            Any: Return value
        """
        try:
            # No parameters for this method
            args = json.dumps({})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="init",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling init: {e}")

    def noop(self) -> None:
        """
        noop method from ABI

        Returns:
            None: Return value
        """
        try:
            # No parameters for this method
            args = json.dumps({})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="noop",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling noop: {e}")

    def echo_bool(self, b: bool) -> bool:
        """
        echo_bool method from ABI

        Args:
            b (bool): Parameter

        Returns:
            bool: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"b": b})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_bool",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_bool: {e}")

    def echo_i32(self, x: int) -> int:
        """
        echo_i32 method from ABI

        Args:
            x (int): Parameter

        Returns:
            int: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_i32",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_i32: {e}")

    def echo_i64(self, x: int) -> int:
        """
        echo_i64 method from ABI

        Args:
            x (int): Parameter

        Returns:
            int: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_i64",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_i64: {e}")

    def echo_u32(self, x: int) -> int:
        """
        echo_u32 method from ABI

        Args:
            x (int): Parameter

        Returns:
            int: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_u32",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_u32: {e}")

    def echo_u64(self, x: int) -> int:
        """
        echo_u64 method from ABI

        Args:
            x (int): Parameter

        Returns:
            int: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_u64",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_u64: {e}")

    def echo_f32(self, x: float) -> float:
        """
        echo_f32 method from ABI

        Args:
            x (float): Parameter

        Returns:
            float: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_f32",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_f32: {e}")

    def echo_f64(self, x: float) -> float:
        """
        echo_f64 method from ABI

        Args:
            x (float): Parameter

        Returns:
            float: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_f64",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_f64: {e}")

    def echo_string(self, s: str) -> str:
        """
        echo_string method from ABI

        Args:
            s (str): Parameter

        Returns:
            str: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"s": s})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_string",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_string: {e}")

    def echo_bytes(self, b: bytes) -> bytes:
        """
        echo_bytes method from ABI

        Args:
            b (bytes): Parameter

        Returns:
            bytes: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"b": b})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="echo_bytes",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling echo_bytes: {e}")

    def opt_u32(self, x: Optional[int]) -> Optional[int]:
        """
        opt_u32 method from ABI

        Args:
            x (Optional[int]): Parameter

        Returns:
            Optional[int]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="opt_u32",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result if result is not None else None
        except ClientError as e:
            raise ClientError(f"Error calling opt_u32: {e}")

    def opt_string(self, x: Optional[str]) -> Optional[str]:
        """
        opt_string method from ABI

        Args:
            x (Optional[str]): Parameter

        Returns:
            Optional[str]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="opt_string",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result if result is not None else None
        except ClientError as e:
            raise ClientError(f"Error calling opt_string: {e}")

    def opt_record(self, p: Optional[Any]) -> Optional[Any]:
        """
        opt_record method from ABI

        Args:
            p (Optional[Any]): Parameter

        Returns:
            Optional[Any]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"p": p})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="opt_record",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result if result is not None else None
        except ClientError as e:
            raise ClientError(f"Error calling opt_record: {e}")

    def opt_id(self, x: Optional[Any]) -> Optional[Any]:
        """
        opt_id method from ABI

        Args:
            x (Optional[Any]): Parameter

        Returns:
            Optional[Any]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="opt_id",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result if result is not None else None
        except ClientError as e:
            raise ClientError(f"Error calling opt_id: {e}")

    def list_u32(self, xs: List[int]) -> List[int]:
        """
        list_u32 method from ABI

        Args:
            xs (List[int]): Parameter

        Returns:
            List[int]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"xs": xs})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="list_u32",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling list_u32: {e}")

    def list_strings(self, xs: List[str]) -> List[str]:
        """
        list_strings method from ABI

        Args:
            xs (List[str]): Parameter

        Returns:
            List[str]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"xs": xs})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="list_strings",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling list_strings: {e}")

    def list_records(self, ps: List[Any]) -> List[Any]:
        """
        list_records method from ABI

        Args:
            ps (List[Any]): Parameter

        Returns:
            List[Any]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"ps": ps})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="list_records",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling list_records: {e}")

    def list_ids(self, xs: List[Any]) -> List[Any]:
        """
        list_ids method from ABI

        Args:
            xs (List[Any]): Parameter

        Returns:
            List[Any]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"xs": xs})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="list_ids",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling list_ids: {e}")

    def map_u32(self, m: Dict[str, int]) -> Dict[str, int]:
        """
        map_u32 method from ABI

        Args:
            m (Dict[str, int]): Parameter

        Returns:
            Dict[str, int]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"m": m})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="map_u32",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling map_u32: {e}")

    def map_list_u32(self, m: Dict[str, List[int]]) -> Dict[str, List[int]]:
        """
        map_list_u32 method from ABI

        Args:
            m (Dict[str, List[int]]): Parameter

        Returns:
            Dict[str, List[int]]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"m": m})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="map_list_u32",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling map_list_u32: {e}")

    def map_record(self, m: Dict[str, Any]) -> Dict[str, Any]:
        """
        map_record method from ABI

        Args:
            m (Dict[str, Any]): Parameter

        Returns:
            Dict[str, Any]: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"m": m})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="map_record",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling map_record: {e}")

    def make_person(self, p: Any) -> Any:
        """
        make_person method from ABI

        Args:
            p (Any): Parameter

        Returns:
            Any: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"p": p})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="make_person",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling make_person: {e}")

    def profile_roundtrip(self, p: Any) -> Any:
        """
        profile_roundtrip method from ABI

        Args:
            p (Any): Parameter

        Returns:
            Any: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"p": p})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="profile_roundtrip",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling profile_roundtrip: {e}")

    def act(self, a: Any) -> int:
        """
        act method from ABI

        Args:
            a (Any): Parameter

        Returns:
            int: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"a": a})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="act",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling act: {e}")

    def roundtrip_id(self, x: Any) -> Any:
        """
        roundtrip_id method from ABI

        Args:
            x (Any): Parameter

        Returns:
            Any: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"x": x})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="roundtrip_id",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling roundtrip_id: {e}")

    def roundtrip_hash(self, h: Any) -> Any:
        """
        roundtrip_hash method from ABI

        Args:
            h (Any): Parameter

        Returns:
            Any: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"h": h})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="roundtrip_hash",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling roundtrip_hash: {e}")

    def may_fail(self, flag: bool) -> int:
        """
        may_fail method from ABI

        Args:
            flag (bool): Parameter

        Returns:
            int: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"flag": flag})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="may_fail",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling may_fail: {e}")

    def find_person(self, name: str) -> Any:
        """
        find_person method from ABI

        Args:
            name (str): Parameter

        Returns:
            Any: Return value
        """
        try:
            # Prepare arguments as JSON
            args = json.dumps({"name": name})
            # Execute the ABI method
            result = self.client.execute_function(
                context_id=self.context_id,
                method="find_person",
                args=args,
                executor_public_key=self.executor_public_key,
            )
            return result
        except ClientError as e:
            raise ClientError(f"Error calling find_person: {e}")
