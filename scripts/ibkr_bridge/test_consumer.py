import unittest

from scripts.ibkr_bridge.consumer import (
    _coerce_bridge_status,
    _recommended_gateway_target,
)


class ConsumerBridgeStatusTests(unittest.TestCase):
    def test_coerce_bridge_status_parses_runtime_client_id_fields(self):
        status = _coerce_bridge_status(
            {
                "state": "running",
                "ts": "123.5",
                "gateway_host": "127.0.0.1",
                "gateway_port": "4002",
                "market_data_type": "3",
                "client_id": "120",
                "configured_client_id": "20",
                "client_id_fallback_engaged": "true",
                "client_id_conflicts": "20,21",
                "subscriptions_active": "3",
            }
        )

        self.assertEqual(status["client_id"], 120)
        self.assertEqual(status["gateway_host"], "127.0.0.1")
        self.assertEqual(status["gateway_port"], 4002)
        self.assertEqual(status["market_data_type"], 3)
        self.assertEqual(status["configured_client_id"], 20)
        self.assertTrue(status["client_id_fallback_engaged"])
        self.assertEqual(status["client_id_conflicts"], [20, 21])
        self.assertEqual(status["subscriptions_active"], 3)
        self.assertEqual(status["ts"], 123.5)

    def test_coerce_bridge_status_handles_empty_conflict_list(self):
        status = _coerce_bridge_status(
            {
                "state": "running",
                "client_id_fallback_engaged": "false",
                "client_id_conflicts": "",
            }
        )

        self.assertFalse(status["client_id_fallback_engaged"])
        self.assertEqual(status["client_id_conflicts"], [])

    def test_recommended_gateway_target_prefers_active_runtime(self):
        target = _recommended_gateway_target(
            {
                "state": "running",
                "gateway_host": "127.0.0.1",
                "gateway_port": 4002,
                "market_data_type": 3,
                "client_id": 120,
                "configured_client_id": 20,
                "client_id_fallback_engaged": True,
                "client_id_conflicts": [20, 21],
                "subscriptions_active": 3,
            }
        )

        self.assertEqual(target["status"], "ready")
        self.assertEqual(target["host"], "127.0.0.1")
        self.assertEqual(target["port"], 4002)
        self.assertEqual(target["client_id"], 120)
        self.assertTrue(target["client_id_fallback_engaged"])
        self.assertIn("fallback", target["message"])

    def test_recommended_gateway_target_handles_absent_bridge(self):
        target = _recommended_gateway_target({"state": "absent"})

        self.assertEqual(target["status"], "bridge_absent")
        self.assertIn("not publishing", target["message"])


if __name__ == "__main__":
    unittest.main()
