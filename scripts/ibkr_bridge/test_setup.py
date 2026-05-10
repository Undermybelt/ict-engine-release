import unittest

from scripts.ibkr_bridge import client_id
from scripts.ibkr_bridge import setup


class GatewaySetupTests(unittest.TestCase):
    def test_scan_common_candidates_prefers_first_reachable(self):
        def fake_ping(host, port, timeout=2.0):
            del host, timeout
            if port == 4002:
                return True, "reachable"
            return False, "unreachable"

        candidates = setup._scan_gateway_candidates("127.0.0.1", None, fake_ping)
        selected = setup._select_gateway_candidate(candidates)

        self.assertEqual([candidate.port for candidate in candidates], [7497, 7496, 4002, 4001])
        self.assertIsNotNone(selected)
        self.assertEqual(selected.port, 4002)
        self.assertEqual(selected.label, "IB Gateway paper")

    def test_scan_explicit_port_limits_candidates(self):
        def fake_ping(host, port, timeout=2.0):
            del host, timeout
            return port == 9001, "checked"

        candidates = setup._scan_gateway_candidates("127.0.0.1", 9001, fake_ping)
        selected = setup._select_gateway_candidate(candidates)

        self.assertEqual(len(candidates), 1)
        self.assertEqual(candidates[0].label, "Custom gateway port 9001")
        self.assertIsNotNone(selected)
        self.assertEqual(selected.port, 9001)

    def test_candidate_client_ids_reserve_offset_range(self):
        self.assertEqual(
            client_id.candidate_client_ids(20, fallback_count=3),
            [20, 120, 121, 122],
        )

    def test_conflict_detection_matches_ibkr_326_text(self):
        self.assertTrue(
            client_id.is_client_id_conflict_error(
                "Error 326: Unable to connect as the client id is already in use."
            )
        )


class ProbeFallbackTests(unittest.IsolatedAsyncioTestCase):
    async def test_probe_fallback_tries_next_client_id(self):
        candidates = [setup.GatewayCandidate("TWS paper", 7497, True, "reachable")]

        async def fake_probe_account(host, port, client_id, timeout):
            del host, port, timeout
            if client_id == 99:
                raise RuntimeError("Error 326: client id is already in use")

            class FakeCaps:
                account_type = "paper"
                n_subaccounts = 1

            return FakeCaps()

        candidate, selected_client_id, caps, attempted_errors = await setup._probe_account_with_fallback(
            "127.0.0.1",
            candidates,
            99,
            8.0,
            probe_account_fn=fake_probe_account,
        )

        self.assertEqual(candidate.port, 7497)
        self.assertEqual(selected_client_id, 199)
        self.assertEqual(caps.account_type, "paper")
        self.assertEqual(len(attempted_errors), 1)


if __name__ == "__main__":
    unittest.main()
