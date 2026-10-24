"""Behavioral fixtures for the source graph; run with Python unittest."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('graph', Path(__file__).with_name('engine-crate-graph.py'))
graph = importlib.util.module_from_spec(spec)
spec.loader.exec_module(graph)


class SourceGraphTests(unittest.TestCase):
    def test_literals_nested_comments_and_grouped_paths(self):
        code = graph.mask_literals('''
// crate::comment::bad
/* crate::bad /* nested */ crate::bad */
const S: &str = r###"crate::bad"###;
const C: char = '}';
use crate::{alpha::{A, B}, beta::C};
fn f<'a>() { crate::gamma::run(); }
''')
        self.assertEqual([name for name, _ in graph.crate_references(code)],
                         ['alpha', 'beta', 'gamma'])

    def test_inline_facade_and_root_client_edges(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'godot/ui-model/src'
            source.mkdir(parents=True)
            (source / 'lib.rs').write_text('''mod alpha;
mod beta;
pub mod ui { pub mod screens { pub use crate::alpha::A; } }
pub struct GameClient;
fn run() { beta::run(); }
''')
            (source / 'alpha.rs').write_text('use crate::ui::strata::FrameStrata;\npub struct A;\n')
            (source / 'beta.rs').write_text('use crate::GameClient;\n')
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            subprocess.run(['git', '-C', str(root), 'add', '.'], check=True)
            subprocess.run(['git', '-C', str(root), '-c', 'user.name=Fixture',
                            '-c', 'user.email=fixture@example.test', 'commit', '-qm', 'Fixture'], check=True)
            result = graph.analyze(root, 'godot/ui-model', '2000-01-01', '2030-01-01')
            self.assertEqual(set(result['modules']), {'@main', 'alpha', 'beta', 'ui'})
            self.assertEqual(result['unresolved'], [])
            self.assertIn(['alpha', 'ui'], result['graphs']['production']['scc'])
            self.assertIn(['@main', 'beta'], result['graphs']['production']['scc'])
            self.assertEqual(result['graphs']['production']['edges']['ui'], ['alpha'])
            self.assertEqual(result['graphs']['production_ui_resolved']['edges']['alpha'], [])
            self.assertEqual(graph.ui_targets('crate::ui::{screens::alpha::A, strata::FrameStrata}', 0), ['alpha'])
            self.assertEqual(graph.ui_targets('crate::{beta::B, ui::{screens::alpha::A, strata::FrameStrata}}', 0), ['alpha'])

    def test_module_ownership_cfg_cycles_and_distinct_commit_churn(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'godot/ui-model/src'
            source.mkdir(parents=True)
            fixtures = {
                'lib.rs': 'mod alpha; mod beta; mod gamma;\npub struct GameClient;\n',
                'alpha.rs': '''use crate::beta::B;
#[cfg(test)]
#[path = "alpha_tests.rs"]
mod tests;
#[path = "alpha_loader.rs"]
mod loader;
''',
                'alpha_loader.rs': 'use crate::beta::B;\n',
                'alpha_tests.rs': 'use crate::gamma::C;\n',
                'beta.rs': 'use crate::alpha::A;\n',
                'gamma.rs': 'pub struct C;\n',
            }
            for name, text in fixtures.items():
                (source / name).write_text(text)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(root), *args], text=True)
            git('init', '-q')
            git('add', '.')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.test',
                'commit', '-qm', 'Fixture')
            result = graph.analyze(root, 'godot/ui-model', '2000-01-01', '2030-01-01')
            self.assertEqual(result['unresolved'], [])
            self.assertEqual(result['graphs']['production']['edges']['alpha'], ['beta'])
            self.assertEqual(result['graphs']['including_tests']['edges']['alpha'], ['beta', 'gamma'])
            self.assertIn(['alpha', 'beta'], result['graphs']['production']['scc'])
            self.assertEqual(result['modules']['alpha']['files'], 3)
            self.assertEqual(result['modules']['alpha']['commits'], 1)
            self.assertEqual(result['modules']['alpha']['test_file_lines'], 1)
            witness = next(edge for edge in result['witnesses'] if edge['to'] == 'gamma')
            self.assertEqual((witness['path'], witness['line'], witness['test_only']),
                             ('godot/ui-model/src/alpha_tests.rs', 1, True))


if __name__ == '__main__':
    unittest.main()
