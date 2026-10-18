"""Synthetic geometry only: these spheres and routes are hand-built, not native proof."""
import math
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bot_hazards import UnreachableRoute, escape_goal, read_areas, route_is_safe


def unreachable(start, goal):
    raise UnreachableRoute("synthetic native refusal")


def area(id=1, center=(0.0, 0.0, 0.0), radius=8.0, caster=77, spell=86636, hostile=True):
    return {"id": id, "caster": caster, "spell": spell, "center": list(center),
            "radius": radius, "hostile": hostile}


def sphere(**kwargs):
    return read_areas({"areas": [area(**kwargs)]})[0]


class FakeRoutes:
    """Returns scripted corners per heading label; records every query."""

    def __init__(self, by_label=None, default=None):
        self.by_label = by_label or {}
        self.default = default or (lambda start, goal: [list(start), list(goal)])
        self.calls = []

    def __call__(self, start, goal, label):
        self.calls.append((list(start), list(goal), label))
        scripted = self.by_label.get(label)
        if isinstance(scripted, Exception):
            raise scripted
        if callable(scripted):
            return scripted(start, goal)
        if scripted is not None:
            return scripted
        return self.default(start, goal)


class ReadAreasTests(unittest.TestCase):
    def test_keeps_hostile_rain_of_fire_and_normalises_geometry(self):
        result = read_areas({"areas": [area(id=4, caster=9, center=(1, 2, 3), radius=8)]})
        self.assertEqual(result, [{"id": 4, "caster": 9, "spell": 86636,
                                   "center": [1.0, 2.0, 3.0], "radius": 8.0, "hostile": True}])

    def test_excludes_other_spells_and_explicit_friendly_areas(self):
        result = read_areas({"areas": [area(id=1, spell=12345), area(id=2, hostile=False),
                                       area(id=3)]})
        self.assertEqual([a["id"] for a in result], [3])

    def test_missing_or_non_list_areas_is_unavailable_not_safe(self):
        for snapshot in [{}, {"areas": None}, {"areas": {}}, None, []]:
            with self.subTest(snapshot=snapshot):
                with self.assertRaisesRegex(ValueError, "areas"):
                    read_areas(snapshot)

    def test_empty_explicit_list_means_no_areas(self):
        self.assertEqual(read_areas({"areas": []}), [])

    def test_unavailable_hostility_raises(self):
        for hostile in [None, 1, "true"]:
            with self.subTest(hostile=hostile):
                with self.assertRaisesRegex(ValueError, "hostile"):
                    read_areas({"areas": [area(hostile=hostile)]})
        entry = area()
        del entry["hostile"]
        with self.assertRaisesRegex(ValueError, "hostile"):
            read_areas({"areas": [entry]})

    def test_malformed_geometry_and_identity_raise(self):
        bad = [("center", None), ("center", [0, 0]), ("center", [0, math.nan, 0]),
               ("center", [0, math.inf, 0]), ("radius", 0), ("radius", -1),
               ("radius", math.nan), ("radius", math.inf), ("radius", None),
               ("id", 0), ("id", -3), ("id", 1.5), ("id", True), ("id", None),
               ("caster", 0), ("caster", "77"), ("spell", None), ("spell", "86636")]
        for key, value in bad:
            with self.subTest(key=key, value=value):
                entry = area()
                entry[key] = value
                with self.assertRaisesRegex(ValueError, key):
                    read_areas({"areas": [entry]})

    def test_missing_required_fact_raises(self):
        for key in ["id", "caster", "spell", "center", "radius"]:
            with self.subTest(key=key):
                entry = area()
                del entry[key]
                with self.assertRaisesRegex(ValueError, key):
                    read_areas({"areas": [entry]})

    def test_non_object_entry_raises(self):
        with self.assertRaises(ValueError):
            read_areas({"areas": [[1, 2, 3]]})


class RouteIsSafeOutsideTests(unittest.TestCase):
    def setUp(self):
        self.areas = [sphere(center=(0, 0, 0), radius=2)]

    def test_segment_crossing_sphere_between_clear_endpoints_is_unsafe(self):
        self.assertFalse(route_is_safe([-5, 0, 0], [[5, 0, 0]], self.areas))

    def test_tangent_segment_touching_inclusive_boundary_is_unsafe(self):
        self.assertFalse(route_is_safe([-5, 0, 2], [[5, 0, 2]], self.areas))

    def test_segment_passing_just_outside_is_safe(self):
        self.assertTrue(route_is_safe([-5, 0, 2.001], [[5, 0, 2.001]], self.areas))

    def test_vertical_offset_counts_in_3d_without_floor_tolerance(self):
        self.assertTrue(route_is_safe([-5, 2.5, 0], [[5, 2.5, 0]], self.areas))
        self.assertFalse(route_is_safe([-5, 1.9, 0], [[5, 1.9, 0]], self.areas))

    def test_later_segment_crossing_is_unsafe(self):
        corners = [[-5, 0, 5], [5, 0, 5], [5, 0, -5], [-5, 0, -5]]
        self.assertTrue(route_is_safe([-5, 0, 6], corners[:3], self.areas))
        self.assertFalse(route_is_safe([-5, 0, 6], [*corners[:2], [-5, 0, -1]], self.areas))

    def test_no_areas_is_safe(self):
        self.assertTrue(route_is_safe([0, 0, 0], [[1, 0, 0]], []))

    def test_invalid_points_and_empty_path_raise(self):
        for here, corners in [([0, 0], [[1, 0, 0]]), ([0, 0, math.nan], [[1, 0, 0]]),
                              ([0, 0, 0], []), ([0, 0, 0], None),
                              ([0, 0, 0], [[1, 0, math.inf]]), ([0, 0, 0], [[1, 0]])]:
            with self.subTest(here=here, corners=corners):
                with self.assertRaises(ValueError):
                    route_is_safe(here, corners, self.areas)


class RouteIsSafeInsideTests(unittest.TestCase):
    def setUp(self):
        self.area = sphere(center=(0, 0, 0), radius=4)

    def test_straight_outward_escape_is_safe(self):
        self.assertTrue(route_is_safe([1, 0, 0], [[3, 0, 0], [5, 0, 0]], [self.area]))

    def test_moving_toward_centre_before_exit_is_unsafe(self):
        self.assertFalse(route_is_safe([1, 0, 0], [[0.5, 0, 0], [5, 0, 0]], [self.area]))

    def test_single_segment_whose_distance_dips_is_unsafe(self):
        self.assertFalse(route_is_safe([2, 0, 0], [[-5, 0, 0.5]], [self.area]))

    def test_tangential_start_then_outward_is_monotonic(self):
        self.assertTrue(route_is_safe([2, 0, 0], [[2, 0, 5]], [self.area]))

    def test_starting_at_centre_any_direction_increases(self):
        self.assertTrue(route_is_safe([0, 0, 0], [[0, 0, -5]], [self.area]))

    def test_reentry_after_exit_is_unsafe(self):
        self.assertFalse(route_is_safe([3, 0, 0], [[5, 0, 0], [5, 0, 3], [-5, 0, 3]],
                                       [self.area]))

    def test_endpoint_still_inside_or_on_boundary_is_unsafe(self):
        self.assertFalse(route_is_safe([1, 0, 0], [[3, 0, 0]], [self.area]))
        self.assertFalse(route_is_safe([1, 0, 0], [[4, 0, 0]], [self.area]))

    def test_touching_other_non_containing_sphere_is_unsafe(self):
        other = sphere(id=2, center=(10, 0, 0), radius=4)
        self.assertTrue(route_is_safe([1, 0, 0], [[5.5, 0, 0]], [self.area, other]))
        self.assertFalse(route_is_safe([1, 0, 0], [[6, 0, 0]], [self.area, other]))


class EscapeGoalTests(unittest.TestCase):
    def test_outside_all_areas_returns_none_without_querying(self):
        routes = FakeRoutes()
        self.assertIsNone(escape_goal([20, 0, 0], [sphere(radius=4)], routes))
        self.assertEqual(routes.calls, [])

    def test_queries_exactly_eight_headings_at_radius_plus_arrival_tolerance(self):
        routes = FakeRoutes()
        escape_goal([1, 3, 0], [sphere(center=(0, 0, 0), radius=4)], routes)
        self.assertEqual(len(routes.calls), 8)
        self.assertEqual(len({label for _, _, label in routes.calls}), 8)
        for k, (start, goal, _) in enumerate(routes.calls):
            angle = math.radians(45 * k)
            self.assertEqual(start, [1, 3, 0])
            self.assertAlmostEqual(goal[0], 4.5 * math.cos(angle))
            self.assertEqual(goal[1], 3)
            self.assertAlmostEqual(goal[2], 4.5 * math.sin(angle))

    def test_headings_offset_from_centre_to_player_ray(self):
        routes = FakeRoutes()
        escape_goal([10, 0, 11], [sphere(center=(10, 0, 10), radius=4)], routes)
        first = routes.calls[0][1]
        self.assertAlmostEqual(first[0], 10)
        self.assertAlmostEqual(first[2], 14.5)

    def test_zero_ray_uses_positive_x(self):
        routes = FakeRoutes()
        escape_goal([2, 1, 2], [sphere(center=(2, 0, 2), radius=4)], routes)
        first = routes.calls[0][1]
        self.assertAlmostEqual(first[0], 6.5)
        self.assertAlmostEqual(first[2], 2)

    def test_straight_outward_heading_wins_and_nominal_goal_is_returned(self):
        routes = FakeRoutes()
        goal = escape_goal([1, 0, 0], [sphere(radius=4)], routes)
        self.assertEqual(goal, routes.calls[0][1])

    def test_shortest_safe_actual_route_wins_over_heading_order(self):
        labels = [label for _, _, label in self._calls([0, 0, 0])]
        routes = FakeRoutes({label: unreachable for label in labels})
        routes.by_label[labels[0]] = [[0, 0, 0], [4.5, 0, 0], [4.5, 0, 1], [4.5, 0, 0]]
        routes.by_label[labels[2]] = [[0, 0, 0], [0, 0, 4.5]]
        goal = escape_goal([0, 0, 0], [sphere(radius=4)], routes)
        self.assertEqual(goal, routes.calls[2][1])

    def test_equal_length_routes_tie_on_heading_order(self):
        labels = [label for _, _, label in self._calls([0, 0, 0])]
        routes = FakeRoutes({label: unreachable for label in labels})
        routes.by_label[labels[2]] = [[0, 0, 0], [0, 0, 4.5]]
        routes.by_label[labels[4]] = [[0, 0, 0], [-4.5, 0, 0]]
        goal = escape_goal([0, 0, 0], [sphere(radius=4)], routes)
        self.assertEqual(goal, routes.calls[2][1])

    def test_unreachable_candidates_are_skipped(self):
        labels = [label for _, _, label in self._calls([1, 0, 0])]
        routes = FakeRoutes({labels[0]: UnreachableRoute("no path"),
                             labels[1]: UnreachableRoute("no path")})
        goal = escape_goal([1, 0, 0], [sphere(radius=4)], routes)
        self.assertEqual(goal, routes.calls[7][1])
        self.assertEqual(len(routes.calls), 8)

    def test_unexpected_query_errors_propagate(self):
        labels = [label for _, _, label in self._calls([1, 0, 0])]
        for error in [RuntimeError("native crashed"), ValueError("bad csv"), OSError("io")]:
            with self.subTest(error=error):
                routes = FakeRoutes({labels[2]: error})
                with self.assertRaises(type(error)):
                    escape_goal([1, 0, 0], [sphere(radius=4)], routes)

    def test_malformed_route_output_propagates(self):
        routes = FakeRoutes(default=lambda start, goal: [[0, math.nan, 0]])
        with self.assertRaises(ValueError):
            escape_goal([1, 0, 0], [sphere(radius=4)], routes)

    def test_partial_or_displaced_projection_is_rejected(self):
        def partial(start, goal):
            return [list(start), [goal[0] * 0.8, goal[1], goal[2] * 0.8]]

        def displaced(start, goal):
            return [list(start), [goal[0] + 0.6, goal[1], goal[2]]]

        labels = [label for _, _, label in self._calls([1, 0, 0])]
        routes = FakeRoutes({labels[0]: partial, labels[1]: displaced})
        goal = escape_goal([1, 0, 0], [sphere(radius=4)], routes)
        self.assertEqual(goal, routes.calls[7][1])

    def test_inward_projection_cannot_allow_native_arrival_inside_the_area(self):
        # Concrete maintained PlayerController boundary: a projected4.1yd endpoint
        # releases arrival at3.9yd, still inside the source4yd sphere.
        from bot_controller import PlayerController
        commands = []
        controller = PlayerController(lambda *argv: commands.append(argv), None,
            lambda start, goal, label: [start, [4.1, 0, 0]], None)
        self.assertTrue(controller.step_toward([3.9, 0, 0], [4.5, 0, 0], "projection-control"))
        self.assertEqual(commands, [])
        def inward(start, goal):
            return [start, [goal[0] * 4.1 / 4.5, goal[1], goal[2] * 4.1 / 4.5]]
        with self.assertRaisesRegex(ValueError, "safe"):
            escape_goal([0, 0, 0], [sphere(radius=4)], FakeRoutes(default=inward))

    def test_projection_within_arrival_tolerance_is_accepted(self):
        def nudged(start, goal):
            return [list(start), [goal[0] + 0.4, goal[1] + 0.3, goal[2]]]

        routes = FakeRoutes(default=nudged)
        goal = escape_goal([1, 0, 0], [sphere(radius=4)], routes)
        self.assertEqual(goal, routes.calls[0][1])

    def test_projected_endpoint_inside_another_sphere_is_rejected(self):
        areas = [sphere(id=1, radius=4), sphere(id=2, center=(6, 0, 0), radius=2)]
        routes = FakeRoutes()
        goal = escape_goal([1, 0, 0], areas, routes)
        self.assertNotEqual(goal, routes.calls[0][1])
        self.assertTrue(route_is_safe([1, 0, 0], [goal], areas))

    def test_deepest_penetration_sphere_drives_headings(self):
        shallow = sphere(id=1, center=(0, 0, 0), radius=4)
        deep = sphere(id=2, center=(3, 0, 0), radius=6)
        routes = FakeRoutes(default=unreachable)
        with self.assertRaises(ValueError):
            escape_goal([3, 0, 0], [shallow, deep], routes)
        self.assertAlmostEqual(routes.calls[0][1][0], 9.5)

    def test_equal_penetration_ties_break_on_lowest_id(self):
        a = sphere(id=9, center=(0, 0, 0), radius=4)
        b = sphere(id=3, center=(4, 0, 0), radius=4)
        routes = FakeRoutes(default=unreachable)
        with self.assertRaises(ValueError):
            escape_goal([2, 0, 0], [a, b], routes)
        # id 3 centre at x=4; ray towards player is -X, so first heading lands at 4-4.5.
        self.assertAlmostEqual(routes.calls[0][1][0], -0.5)

    def test_no_safe_candidate_raises_without_direct_fallback(self):
        routes = FakeRoutes(default=lambda start, goal: [list(start), [0, 0, 0], list(goal)])
        with self.assertRaisesRegex(ValueError, "safe"):
            escape_goal([1, 0, 0], [sphere(radius=4)], routes)

    def test_multiple_overlapping_volumes_with_no_exit_raise(self):
        ring = [sphere(id=1, radius=4)] + [
            sphere(id=10 + k, radius=1.5,
                   center=(4.5 * math.cos(math.radians(45 * k)), 0,
                           4.5 * math.sin(math.radians(45 * k))))
            for k in range(8)]
        with self.assertRaisesRegex(ValueError, "safe"):
            escape_goal([1, 0, 0], ring, FakeRoutes())

    def _calls(self, here):
        routes = FakeRoutes()
        escape_goal(here, [sphere(radius=4)], routes)
        return routes.calls


if __name__ == "__main__":
    unittest.main()
