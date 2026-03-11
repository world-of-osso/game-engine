ui.waitForFrame("UsernameInput", 5.0);
ui.click("UsernameInput");
ui.type(env.LOGIN_USER);
ui.click("PasswordInput");
ui.type(env.LOGIN_PASS);
ui.click("ConnectButton");
ui.waitForState("CharSelect", 10.0);
ui.dumpUiTree();
