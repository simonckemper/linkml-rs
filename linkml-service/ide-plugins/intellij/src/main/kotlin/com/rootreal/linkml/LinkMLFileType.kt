package com.rootreal.linkml

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.openapi.fileTypes.FileType
import javax.swing.Icon
import com.intellij.openapi.util.IconLoader

/**
 * LinkML file type definition
 */
class LinkMLFileType private constructor() : LanguageFileType(LinkMLLanguage.INSTANCE) {

    override fun getName(): String = "LinkML Schema"

    override fun getDescription(): String = "LinkML schema file"

    override fun getDefaultExtension(): String = "linkml.yaml"

    override fun getIcon(): Icon = LinkMLIcons.FILE

    companion object {
        @JvmField
        val INSTANCE = LinkMLFileType()
    }
}

/**
 * LinkML Language definition
 */
object LinkMLLanguage : com.intellij.lang.Language("LinkML") {
    @JvmField
    val INSTANCE = this

    override fun getDisplayName(): String = "LinkML"

    override fun isCaseSensitive(): Boolean = true
}

/**
 * LinkML Icons
 */
object LinkMLIcons {
    @JvmField
    val FILE = IconLoader.getIcon("/icons/linkml.svg", LinkMLIcons::class.java)

    @JvmField
    val CLASS = IconLoader.getIcon("/icons/class.svg", LinkMLIcons::class.java)

    @JvmField
    val ATTRIBUTE = IconLoader.getIcon("/icons/attribute.svg", LinkMLIcons::class.java)

    @JvmField
    val ENUM = IconLoader.getIcon("/icons/enum.svg", LinkMLIcons::class.java)

    @JvmField
    val TYPE = IconLoader.getIcon("/icons/type.svg", LinkMLIcons::class.java)

    @JvmField
    val SLOT = IconLoader.getIcon("/icons/slot.svg", LinkMLIcons::class.java)
}
