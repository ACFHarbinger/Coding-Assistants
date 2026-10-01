package com.codingassistants.remotelauncher.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.codingassistants.remotelauncher.sync.RestoreException
import com.codingassistants.remotelauncher.sync.RestoreResult
import com.codingassistants.remotelauncher.sync.SnapshotRestore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

data class SnapshotUiState(
    val busy: Boolean = false,
    val result: RestoreResult? = null,
    val error: String? = null,
    val homePath: String = "",
)

class SnapshotRestoreViewModel(
    application: Application,
) : AndroidViewModel(application) {
    private val _state = MutableStateFlow(SnapshotUiState(homePath = companionHome().absolutePath))
    val state: StateFlow<SnapshotUiState> = _state.asStateFlow()

    fun companionHome(): File {
        val context = getApplication<Application>()
        return context.getExternalFilesDir(null) ?: context.filesDir
    }

    fun restore() {
        viewModelScope.launch {
            _state.value = _state.value.copy(busy = true, error = null, result = null)
            try {
                val home = companionHome()
                val result =
                    withContext(Dispatchers.IO) {
                        SnapshotRestore.restore(home)
                    }
                _state.value =
                    _state.value.copy(
                        busy = false,
                        result = result,
                        homePath = home.absolutePath,
                    )
            } catch (error: RestoreException) {
                _state.value =
                    _state.value.copy(
                        busy = false,
                        error = error.message,
                    )
            } catch (error: Exception) {
                _state.value =
                    _state.value.copy(
                        busy = false,
                        error = error.message ?: "restore failed",
                    )
            }
        }
    }
}
