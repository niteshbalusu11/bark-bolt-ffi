package io.github.niteshbalusu11.bark.example

import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.assertExists
import org.junit.Rule
import org.junit.Test

class WalletUITest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()
    @Test fun onboardingShowsRegtestAndWalletAction() {
        compose.onNodeWithText("Bark Wallet").assertExists()
        compose.onNodeWithText("RegTest · local funds only").assertExists()
        compose.onNodeWithTag("open-wallet").assertExists()
    }
}
