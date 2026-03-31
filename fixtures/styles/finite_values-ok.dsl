workspace {
    views {
        styles {
            element "Shape Box" { shape Box }
            element "Shape RoundedBox" { shape RoundedBox }
            element "Shape Circle" { shape Circle }
            element "Shape Ellipse" { shape Ellipse }
            element "Shape Hexagon" { shape Hexagon }
            element "Shape Diamond" { shape Diamond }
            element "Shape Cylinder" { shape Cylinder }
            element "Shape Bucket" { shape Bucket }
            element "Shape Pipe" { shape Pipe }
            element "Shape Person" { shape Person }
            element "Shape Robot" { shape Robot }
            element "Shape Folder" { shape Folder }
            element "Shape WebBrowser" { shape WebBrowser }
            element "Shape Window" { shape Window }
            element "Shape Terminal" { shape Terminal }
            element "Shape Shell" { shape Shell }
            element "Shape MobileDevicePortrait" { shape MobileDevicePortrait }
            element "Shape MobileDeviceLandscape" { shape MobileDeviceLandscape }
            element "Shape Component" { shape Component }

            element "Border Solid" { border Solid }
            element "Border Dashed" { border Dashed }
            element "Border Dotted" { border Dotted }

            element "Icon Top" { iconPosition Top }
            element "Icon Bottom" { iconPosition Bottom }
            element "Icon Left" { iconPosition Left }

            element "Element Metadata True" { metadata true }
            element "Element Metadata False" { metadata false }
            element "Element Description True" { description true }
            element "Element Description False" { description false }

            relationship "LineStyle Solid" { style Solid }
            relationship "LineStyle Dashed" { style Dashed }
            relationship "LineStyle Dotted" { style Dotted }

            relationship "Routing Direct" { routing Direct }
            relationship "Routing Curved" { routing Curved }
            relationship "Routing Orthogonal" { routing Orthogonal }

            relationship "Relationship Dashed True" { dashed true }
            relationship "Relationship Dashed False" { dashed false }
            relationship "Relationship Jump True" { jump true }
            relationship "Relationship Jump False" { jump false }
            relationship "Relationship Metadata True" { metadata true }
            relationship "Relationship Metadata False" { metadata false }
            relationship "Relationship Description True" { description true }
            relationship "Relationship Description False" { description false }
        }
    }
}
